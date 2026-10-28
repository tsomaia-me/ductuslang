#![forbid(unsafe_code)]

#[path = "../../../../crates/compiler/tests/robustness_corpus/mod.rs"]
mod robustness_corpus;

use std::io::{BufWriter, Write};
use std::process::ExitCode;

use symphonyscript::Source;
use symphonyscript::syntax::lex;
use symphonyscript_evolve::VOCABULARY_NAMES;

const SOURCE_PATH: &str = "main.sym";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("evolve-mutants: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let corpora = robustness_corpus::corpora();
    if corpora.len() != VOCABULARY_NAMES.len() {
        return Err(format!(
            "the robustness corpus holds {} vocabularies and evolve names {}",
            corpora.len(),
            VOCABULARY_NAMES.len()
        ));
    }
    let mut out = BufWriter::new(std::io::stdout().lock());
    for (name, (vocabulary, texts)) in VOCABULARY_NAMES.into_iter().zip(corpora) {
        for text in &texts {
            let source = Source {
                path: SOURCE_PATH.into(),
                text: text.clone(),
            };
            let tokens: Vec<(usize, usize)> = lex(&source, &vocabulary)
                .tokens
                .iter()
                .filter_map(|token| {
                    let start = usize::try_from(token.range.start).ok()?;
                    let end = usize::try_from(token.range.end).ok()?;
                    Some((start, end))
                })
                .collect();
            for mutant in robustness_corpus::mutants(text, &tokens) {
                let line = serde_json::json!({
                    "vocabulary": name,
                    "sources": [{"path": SOURCE_PATH, "text": mutant}],
                });
                writeln!(out, "{line}").map_err(|error| error.to_string())?;
            }
        }
    }
    out.flush().map_err(|error| error.to_string())
}
