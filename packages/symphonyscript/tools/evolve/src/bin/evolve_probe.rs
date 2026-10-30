#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use symphonyscript::{Compiler, Source};
use symphonyscript_evolve::{
    PROBE_TAGS, PROBED_PATH, ProbeProgram, ProbeSummary, Probed, named_vocabulary, probe_block,
    probe_programs, probed, tagged_vocabulary,
};

const USAGE: &str = "usage: evolve-probe [--budget <seconds>] [--jobs <n>] [--values] [--summary] <programs file>... | evolve-probe --tags";
const DEFAULT_BUDGET: Duration = Duration::from_secs(30);
const DEFAULT_JOBS: usize = 4;
const GRACE: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(5);

struct Options {
    budget: Duration,
    jobs: usize,
    values: bool,
    summary: bool,
    tags: bool,
    program: Option<String>,
    files: Vec<String>,
}

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {}));
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("evolve-probe: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let options = options(std::env::args().skip(1))?;
    if options.tags {
        for (tag, vocabulary) in PROBE_TAGS {
            println!("{tag} {vocabulary}");
        }
        return Ok(());
    }
    match &options.program {
        Some(tag) => probe_one(tag, options.budget),
        None => probe_all(&options),
    }
}

fn options(raw: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        budget: DEFAULT_BUDGET,
        jobs: DEFAULT_JOBS,
        values: false,
        summary: false,
        tags: false,
        program: None,
        files: Vec::new(),
    };
    let mut raw = raw.into_iter();
    while let Some(argument) = raw.next() {
        match argument.as_str() {
            "--budget" => {
                let value = raw
                    .next()
                    .ok_or_else(|| "--budget takes a number of seconds".to_string())?;
                options.budget = value
                    .parse()
                    .ok()
                    .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
                    .ok_or_else(|| {
                        format!("--budget takes a non-negative number of seconds, not `{value}`")
                    })?;
            }
            "--jobs" => {
                let value = raw
                    .next()
                    .ok_or_else(|| "--jobs takes a number of processes".to_string())?;
                options.jobs = value
                    .parse()
                    .ok()
                    .filter(|jobs| *jobs > 0)
                    .ok_or_else(|| format!("--jobs takes a positive number, not `{value}`"))?;
            }
            "--values" => options.values = true,
            "--summary" => options.summary = true,
            "--tags" => options.tags = true,
            "--program" => {
                options.program = Some(
                    raw.next()
                        .ok_or_else(|| "--program takes a tag".to_string())?,
                );
            }
            _ if argument.starts_with("--") => {
                return Err(format!("unknown option `{argument}`; {USAGE}"));
            }
            _ => options.files.push(argument),
        }
    }
    Ok(options)
}

fn probe_one(tag: &str, budget: Duration) -> Result<(), String> {
    let name = tagged_vocabulary(tag).ok_or_else(|| format!("unknown tag `{tag}`"))?;
    let vocabulary = named_vocabulary(name)?;
    let compiler = Compiler::new(&vocabulary)
        .map_err(|failed| format!("the vocabulary `{tag}` does not compile: {failed:?}"))?;
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|error| format!("stdin: {error}"))?;
    let sources = vec![Source {
        path: PROBED_PATH.to_string(),
        text: text.clone(),
    }];
    let outcome = probed(budget, &text, move || compiler.check(&sources));
    let encoded = serde_json::to_string(&outcome).map_err(|error| error.to_string())?;
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{encoded}")
        .and_then(|()| stdout.flush())
        .map_err(|error| format!("stdout: {error}"))?;
    std::process::exit(0)
}

fn probe_all(options: &Options) -> Result<(), String> {
    if options.files.is_empty() {
        return Err(USAGE.to_string());
    }
    let mut programs = Vec::new();
    for file in &options.files {
        let content = std::fs::read_to_string(file).map_err(|error| format!("{file}: {error}"))?;
        programs.extend(probe_programs(file, &content));
    }
    for program in &programs {
        if tagged_vocabulary(&program.tag).is_none() {
            return Err(format!(
                "{}:{}: unknown tag `#{}`; the tags are {}",
                program.file,
                program.line,
                program.tag,
                PROBE_TAGS.map(|(tag, _)| format!("#{tag}")).join(", ")
            ));
        }
    }
    let executable =
        std::env::current_exe().map_err(|error| format!("the probe's own path: {error}"))?;
    let next = AtomicUsize::new(0);
    let (sender, receiver) = mpsc::channel();
    let mut summary = ProbeSummary::default();
    thread::scope(|scope| {
        for _ in 0..options.jobs.min(programs.len()) {
            let sender = sender.clone();
            let (next, programs, executable) = (&next, &programs, &executable);
            scope.spawn(move || {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(program) = programs.get(index) else {
                        break;
                    };
                    let outcome = in_process(executable, program, options.budget);
                    if sender.send((index, outcome)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        let mut pending = BTreeMap::new();
        let mut printed = 0;
        let mut stdout = std::io::stdout().lock();
        for (index, outcome) in receiver {
            pending.insert(index, outcome);
            while let Some(outcome) = pending.remove(&printed) {
                let program = &programs[printed];
                summary.add(program, &outcome, options.values);
                let block = probe_block(printed + 1, program, &outcome, options.values);
                let _ = write!(stdout, "{block}").and_then(|()| stdout.flush());
                printed += 1;
            }
        }
    });
    if options.summary {
        println!("summary: {summary}");
    }
    Ok(())
}

fn in_process(executable: &std::path::Path, program: &ProbeProgram, budget: Duration) -> Probed {
    let spawned = Command::new(executable)
        .args([
            "--program",
            &program.tag,
            "--budget",
            &budget.as_secs_f64().to_string(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            return Probed::Panicked {
                message: format!("could not start the probe process: {error}"),
            };
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(program.text.as_bytes());
    }
    let stdout = child.stdout.take().map(read_all);
    let stderr = child.stderr.take().map(read_all);
    let deadline = Instant::now() + budget + GRACE;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(POLL),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let output = stdout
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    let errors = stderr
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    let Some(status) = status else {
        return Probed::TimedOut;
    };
    serde_json::from_str(output.trim()).unwrap_or_else(|_| Probed::Panicked {
        message: format!(
            "the probe process ended with {status}: {}",
            errors.lines().last().unwrap_or("no message")
        ),
    })
}

fn read_all(mut stream: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut text = String::new();
        let _ = stream.read_to_string(&mut text);
        text
    })
}
