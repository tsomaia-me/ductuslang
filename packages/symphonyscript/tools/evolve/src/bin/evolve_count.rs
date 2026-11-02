#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::process::ExitCode;

use serde_json::json;
use symphonyscript_evolve::{
    Program, arguments, expanded_items, fixture_program, named_vocabulary, syntax_nodes,
};

const USAGE: &str = "usage: evolve-count [--vocabulary <name>] [--nodes] <programs.jsonl> <counts.jsonl> | evolve-count --fixture";
const FIXTURE: &str = "--fixture";
const NODES: &str = "--nodes";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("evolve-count: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw == [FIXTURE] {
        let encoded =
            serde_json::to_string(&fixture_program()).map_err(|error| error.to_string())?;
        println!("{encoded}");
        return Ok(());
    }
    let nodes_only = raw.iter().any(|argument| argument == NODES);
    let arguments = arguments(raw.into_iter().filter(|argument| argument != NODES))?;
    let [input, output] = arguments.positional.as_slice() else {
        return Err(USAGE.to_string());
    };
    let vocabulary = named_vocabulary(&arguments.vocabulary)?;
    let reader = BufReader::new(File::open(input).map_err(|error| format!("{input}: {error}"))?);
    let mut writer =
        BufWriter::new(File::create(output).map_err(|error| format!("{output}: {error}"))?);
    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| format!("{input}: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let program: Program = serde_json::from_str(&line)
            .map_err(|error| format!("{input}:{}: {error}", index + 1))?;
        let items = if nodes_only {
            None
        } else {
            expanded_items(&program, &vocabulary).map_err(|failed| {
                format!(
                    "the vocabulary `{}` does not compile: {failed:?}",
                    arguments.vocabulary
                )
            })?
        };
        let counted = json!({
            "id": program.id,
            "syntax_nodes": syntax_nodes(&program, &vocabulary),
            "expanded_items": items,
        });
        writeln!(writer, "{counted}").map_err(|error| format!("{output}: {error}"))?;
    }
    writer.flush().map_err(|error| format!("{output}: {error}"))
}
