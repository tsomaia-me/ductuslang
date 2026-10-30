#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::ExitCode;

use serde_json::json;
use symphonyscript_evolve::{
    Checker, Guarded, Program, abandoned, arguments, named_vocabulary, restarted, resumed_output,
    running_compiles,
};

const USAGE: &str =
    "usage: evolve-time [--budget <seconds>] [--vocabulary <name>] <programs.jsonl> <times.jsonl>";

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {}));
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("evolve-time: {message}");
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
    let checker = Checker::with_vocabulary(arguments.budget, vocabulary).map_err(|failed| {
        format!(
            "the vocabulary `{}` does not compile: {failed:?}",
            arguments.vocabulary
        )
    })?;
    let reader = BufReader::new(File::open(input).map_err(|error| format!("{input}: {error}"))?);
    let mut writer = BufWriter::new(
        resumed_output(output, arguments.resume_at)
            .map_err(|error| format!("{output}: {error}"))?,
    );
    for (index, line) in reader.lines().enumerate().skip(arguments.resume_at) {
        let line = line.map_err(|error| format!("{input}: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let program: Program = serde_json::from_str(&line)
            .map_err(|error| format!("{input}:{}: {error}", index + 1))?;
        let running = running_compiles();
        let timed = match checker.compile(&program) {
            Guarded::Finished { value, elapsed, .. } => {
                let mut codes: Vec<&str> = value
                    .iter()
                    .map(|diagnostic| diagnostic.code.as_str())
                    .collect();
                codes.sort_unstable();
                codes.dedup();
                json!({
                    "id": program.id,
                    "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                    "diagnostics": value.len(),
                    "codes": codes,
                    "running_compiles": running,
                })
            }
            Guarded::Panicked { message } => {
                json!({"id": program.id, "elapsed_ms": null, "panic": message, "running_compiles": running})
            }
            Guarded::TimedOut => {
                json!({"id": program.id, "elapsed_ms": null, "running_compiles": running})
            }
        };
        writeln!(writer, "{timed}").map_err(|error| format!("{output}: {error}"))?;
        writer
            .flush()
            .map_err(|error| format!("{output}: {error}"))?;
        if abandoned() {
            return Err(restarted(index + 1));
        }
    }
    Ok(())
}
