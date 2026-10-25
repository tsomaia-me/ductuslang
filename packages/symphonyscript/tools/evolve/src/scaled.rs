use symphonyscript_core::Vocabulary;

use crate::program::{Program, ProgramSource};
use crate::renamed::renamed_copies;

pub fn scaled(program: &Program, vocabulary: &Vocabulary, factor: usize) -> Program {
    let copies = renamed_copies(program, vocabulary, factor.saturating_sub(1));
    let sources = program
        .sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            let mut text = source.text.clone();
            for copy in copies.iter().filter_map(|copy| copy.sources.get(index)) {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
                text.push_str(&copy.text);
            }
            ProgramSource {
                path: source.path.clone(),
                text,
            }
        })
        .collect();
    program.with_sources(sources)
}
