use symphonyscript::Source;
use symphonyscript::syntax::{SyntaxKind, parse};
use symphonyscript_core::Vocabulary;

use crate::program::ProgramSource;

const DECLARATIONS: &[SyntaxKind] = &[
    SyntaxKind::InputDecl,
    SyntaxKind::LetDecl,
    SyntaxKind::FnDecl,
    SyntaxKind::PatternDecl,
];

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Item {
    pub text: String,
    pub declaration: bool,
}

pub fn top_level_items(source: &ProgramSource, vocabulary: &Vocabulary) -> Vec<Item> {
    let parsed = parse(
        &Source {
            path: source.path.clone(),
            text: source.text.clone(),
        },
        vocabulary,
    );
    let mut starts: Vec<(usize, bool)> = parsed
        .root()
        .children()
        .filter_map(|item| {
            let start = usize::try_from(item.range().start).ok()?;
            let at_line_start = start == 0
                || source
                    .text
                    .get(..start)
                    .is_some_and(|before| before.ends_with('\n'));
            at_line_start.then_some((start, DECLARATIONS.contains(&item.kind())))
        })
        .collect();
    starts.dedup_by_key(|(start, _)| *start);
    let mut items = Vec::new();
    let mut start = 0;
    let mut declaration = false;
    for (boundary, next_declaration) in starts.into_iter().chain([(source.text.len(), false)]) {
        if let Some(text) = source.text.get(start..boundary)
            && !text.is_empty()
        {
            items.push(Item {
                text: text.to_string(),
                declaration,
            });
        }
        start = boundary;
        declaration = next_declaration;
    }
    items
}

pub fn joined(items: &[Item]) -> String {
    let mut text = String::new();
    for item in items {
        text.push_str(&item.text);
        if !item.text.ends_with('\n') {
            text.push('\n');
        }
    }
    text
}
