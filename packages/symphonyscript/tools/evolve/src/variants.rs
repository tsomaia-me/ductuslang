use symphonyscript_core::Vocabulary;

use crate::items::{joined, top_level_items};
use crate::program::{Program, ProgramSource};
use crate::shuffled::shuffled;

const SHUFFLE_SEEDS: [u64; 3] = [1, 2, 3];
const SINGLE_FILE_PATH: &str = "main.sym";

pub fn reordered(program: &Program, vocabulary: &Vocabulary) -> Vec<Program> {
    let ordered = program.ordered_sources();
    let items: Vec<String> = ordered
        .iter()
        .flat_map(|source| top_level_items(source, vocabulary))
        .collect();
    if items.len() < 2 {
        return Vec::new();
    }
    let path = ordered
        .first()
        .map_or(SINGLE_FILE_PATH, |source| source.path.as_str())
        .to_string();
    let single = |items: Vec<String>| {
        program.with_sources(vec![ProgramSource {
            path: path.clone(),
            text: joined(&items),
        }])
    };
    let mut variants = vec![single(items.iter().rev().cloned().collect())];
    variants.extend(
        SHUFFLE_SEEDS
            .iter()
            .map(|seed| single(shuffled(&items, *seed))),
    );
    variants.push(
        program.with_sources(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| ProgramSource {
                    path: format!("item-{index:06}.sym"),
                    text: joined(std::slice::from_ref(item)),
                })
                .collect(),
        ),
    );
    variants
}

pub fn with_noise(program: &Program) -> Program {
    let mut counter = 0usize;
    let name = loop {
        let candidate = format!("evolvenoise{counter}");
        if program
            .sources
            .iter()
            .all(|source| !source.text.contains(&candidate))
        {
            break candidate;
        }
        counter += 1;
    };
    let line = format!("let {name} = 0\n");
    let last = program
        .ordered_sources()
        .last()
        .map(|source| source.path.clone());
    let mut sources = program.sources.clone();
    match last.and_then(|path| sources.iter_mut().find(|source| source.path == path)) {
        Some(source) => {
            if !source.text.is_empty() && !source.text.ends_with('\n') {
                source.text.push('\n');
            }
            source.text.push_str(&line);
        }
        None => sources.push(ProgramSource {
            path: SINGLE_FILE_PATH.to_string(),
            text: line,
        }),
    }
    program.with_sources(sources)
}
