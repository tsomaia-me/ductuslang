use symphonyscript_core::Vocabulary;

use crate::items::{Item, joined, top_level_items};
use crate::program::{Program, ProgramSource};
use crate::shuffled::shuffled;

const SHUFFLE_SEEDS: [u64; 3] = [1, 2, 3];
const SINGLE_FILE_PATH: &str = "main.sym";

pub fn reordered(program: &Program, vocabulary: &Vocabulary) -> Vec<Program> {
    let ordered = program.ordered_sources();
    let items: Vec<Item> = ordered
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
    let single = |permuted: Vec<Item>| {
        program.with_sources(vec![ProgramSource {
            path: path.clone(),
            text: joined(&placements_in_place(&items, permuted)),
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

fn placements_in_place(original: &[Item], permuted: Vec<Item>) -> Vec<Item> {
    let mut placements = original.iter().filter(|item| !item.declaration);
    permuted
        .into_iter()
        .map(|item| {
            if item.declaration {
                item
            } else {
                placements.next().cloned().unwrap_or(item)
            }
        })
        .collect()
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
    let last = program
        .ordered_sources()
        .last()
        .map_or(SINGLE_FILE_PATH, |source| source.path.as_str());
    let stem = last.strip_suffix(".sym").unwrap_or(last);
    let mut sources = program.sources.clone();
    sources.push(ProgramSource {
        path: format!("{stem}~noise.sym"),
        text: format!("let {name} = 0\n"),
    });
    program.with_sources(sources)
}
