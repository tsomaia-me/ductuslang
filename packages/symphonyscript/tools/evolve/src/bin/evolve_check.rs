#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::ExitCode;
use std::time::Instant;

use symphonyscript_evolve::{Checker, Program, arguments, named_vocabulary};

const USAGE: &str = "usage: evolve-check [--budget <seconds>] [--memory-bound <bytes per source byte>] [--vocabulary <name>] <programs.jsonl> <verdicts.jsonl>";

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {}));
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("evolve-check: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = arguments(std::env::args().skip(1))?;
    let [input, output] = arguments.positional.as_slice() else {
        return Err(USAGE.to_string());
    };
    let vocabulary = named_vocabulary(&arguments.vocabulary)?;
    let checker = Checker::with_vocabulary(arguments.budget, vocabulary)
        .map_err(|failed| {
            format!(
                "the vocabulary `{}` does not compile: {failed:?}",
                arguments.vocabulary
            )
        })?
        .with_memory_bound(arguments.memory_bound);
    let reader = BufReader::new(File::open(input).map_err(|error| format!("{input}: {error}"))?);
    let mut writer =
        BufWriter::new(File::create(output).map_err(|error| format!("{output}: {error}"))?);
    let started = Instant::now();
    let mut checked = 0usize;
    let mut flagged = 0usize;
    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| format!("{input}: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let program: Program = match serde_json::from_str(&line) {
            Ok(program) => program,
            Err(error) => {
                eprintln!("evolve-check: {input}:{}: {error}", index + 1);
                continue;
            }
        };
        let verdict = checker.verdict(&program);
        checked += 1;
        if !verdict.signals.is_empty() {
            flagged += 1;
        }
        let encoded = serde_json::to_string(&verdict).map_err(|error| error.to_string())?;
        writeln!(writer, "{encoded}").map_err(|error| format!("{output}: {error}"))?;
    }
    writer
        .flush()
        .map_err(|error| format!("{output}: {error}"))?;
    let seconds = started.elapsed().as_secs_f64();
    eprintln!(
        "evolve-check: {checked} programs, {flagged} with signals, {seconds:.3} s, {:.1} programs/s",
        if seconds > 0.0 {
            checked as f64 / seconds
        } else {
            0.0
        }
    );
    Ok(())
}
