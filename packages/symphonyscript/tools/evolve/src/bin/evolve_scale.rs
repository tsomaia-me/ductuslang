#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::ExitCode;
use std::sync::Arc;

use serde_json::{Value, json};
use symphonyscript::syntax::parse;
use symphonyscript::{Compiler, Diagnostic, Selector};
use symphonyscript_evolve::{
    Engine, Guarded, Program, arguments, copied_codes, differing_codes, guarded, named_vocabulary,
    scaled,
};

const USAGE: &str = "usage: evolve-scale [--budget <seconds>] [--vocabulary <name>] <programs.jsonl> <scales.jsonl>";
const FACTORS: [usize; 4] = [1, 2, 4, 8];

fn main() -> ExitCode {
    std::panic::set_hook(Box::new(|_| {}));
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("evolve-scale: {message}");
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
    let compiler = |selector| {
        Compiler::with_limits(&vocabulary, arguments.limits)
            .map(|mut compiler| {
                compiler.select(selector);
                Arc::new(compiler)
            })
            .map_err(|failed| format!("the vocabulary does not compile: {failed:?}"))
    };
    let selected = compiler(Selector::Default)?;
    let old = compiler(Selector::OldEngine)?;
    let reader = BufReader::new(File::open(input).map_err(|error| format!("{input}: {error}"))?);
    let mut writer =
        BufWriter::new(File::create(output).map_err(|error| format!("{output}: {error}"))?);
    for line in reader.lines() {
        let line = line.map_err(|error| format!("{input}: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let program: Program = serde_json::from_str(&line).map_err(|error| error.to_string())?;
        let mut original = None;
        let mut sizes = Vec::new();
        for factor in FACTORS {
            let copy = scaled(&program, &vocabulary, factor);
            let sources = copy.compiler_sources();
            let front_vocabulary = vocabulary.clone();
            let front_sources = sources.clone();
            let front = guarded(arguments.budget, move || {
                front_sources
                    .iter()
                    .map(|source| parse(source, &front_vocabulary).diagnostics.len())
                    .sum::<usize>()
            });
            let by_default = measured(&selected, &sources, arguments.budget);
            let by_old = measured(&old, &sources, arguments.budget);
            let differing = match (&original, &by_default.0) {
                (None, Some(diagnostics)) => {
                    original = Some(diagnostics.clone());
                    Some(Vec::new())
                }
                (Some(first), Some(diagnostics)) => {
                    let mut drawn: Vec<String> = diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.as_str().to_string())
                        .collect();
                    drawn.sort();
                    Some(differing_codes(
                        &copied_codes(&program, first, factor),
                        &drawn,
                    ))
                }
                _ => None,
            };
            sizes.push(json!({
                "factor": factor,
                "source_bytes": copy.size(),
                "copied": differing.as_ref().map(Vec::is_empty),
                "differing": differing,
                "front": work_of(&front),
                "default": by_default.1,
                "old": by_old.1,
            }));
        }
        let record = json!({ "id": program.id, "sizes": sizes });
        writeln!(writer, "{record}").map_err(|error| format!("{output}: {error}"))?;
        writer
            .flush()
            .map_err(|error| format!("{output}: {error}"))?;
    }
    Ok(())
}

fn measured(
    compiler: &Arc<Compiler>,
    sources: &[symphonyscript::Source],
    budget: std::time::Duration,
) -> (Option<Vec<Diagnostic>>, Value) {
    let compiler = Arc::clone(compiler);
    let sources = sources.to_vec();
    match guarded(budget, move || compiler.check_by_engine(&sources)) {
        Guarded::Finished {
            value: (diagnostics, selector),
            elapsed,
            work,
        } => {
            let engine = Engine::of(selector);
            let record = json!({
                "engine": engine,
                "allocations": work.allocations,
                "peak_bytes": work.peak_bytes,
                "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
                "diagnostics": diagnostics.len(),
            });
            (Some(diagnostics), record)
        }
        Guarded::Panicked { message } => (None, json!({ "panic": message })),
        Guarded::TimedOut => (None, json!({ "timeout": true })),
    }
}

fn work_of<T>(guarded: &Guarded<T>) -> Value {
    match guarded {
        Guarded::Finished { work, .. } => {
            json!({ "allocations": work.allocations, "peak_bytes": work.peak_bytes })
        }
        Guarded::Panicked { .. } | Guarded::TimedOut => Value::Null,
    }
}
