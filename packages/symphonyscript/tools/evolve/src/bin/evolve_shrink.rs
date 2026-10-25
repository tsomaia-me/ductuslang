#![forbid(unsafe_code)]

use std::process::ExitCode;

use serde::Serialize;
use symphonyscript_evolve::{
    Checker, Program, ProgramSource, Signal, arguments, dedup_key, shrunk,
};

const USAGE: &str = "usage: evolve-shrink [--budget <seconds>] [--memory-bound <bytes per source byte>] <program.json> <signal>";

#[derive(Serialize)]
struct Shrunk {
    id: serde_json::Value,
    signal: Signal,
    key: String,
    codes: Vec<String>,
    sources: Vec<ProgramSource>,
}

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {}));
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("evolve-shrink: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = arguments(std::env::args().skip(1))?;
    let [path, signal] = arguments.positional.as_slice() else {
        return Err(USAGE.to_string());
    };
    let signal: Signal = signal.parse().map_err(|error| format!("{error}"))?;
    let text = std::fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    let program: Program =
        serde_json::from_str(&text).map_err(|error| format!("{path}: {error}"))?;
    let checker = Checker::new(arguments.budget)
        .map_err(|failed| format!("the fixture vocabulary does not compile: {failed:?}"))?
        .with_memory_bound(arguments.memory_bound);
    let finding = checker
        .finding(&program, signal)
        .ok_or_else(|| format!("the program does not show the signal `{signal}`"))?;
    let smallest = shrunk(&checker, &program, &finding);
    let result = Shrunk {
        id: smallest.id.clone(),
        key: dedup_key(&finding, &smallest),
        signal,
        codes: finding.codes,
        sources: smallest.sources,
    };
    let encoded = serde_json::to_string(&result).map_err(|error| error.to_string())?;
    println!("{encoded}");
    Ok(())
}
