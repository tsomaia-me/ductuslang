use symphonyscript::Source;
use symphonyscript::syntax::parse;
use symphonyscript_core::Vocabulary;

use crate::program::ProgramSource;

pub fn top_level_items(source: &ProgramSource, vocabulary: &Vocabulary) -> Vec<String> {
    let parsed = parse(
        &Source {
            path: source.path.clone(),
            text: source.text.clone(),
        },
        vocabulary,
    );
    let mut boundaries: Vec<usize> = parsed
        .root()
        .children()
        .filter_map(|item| usize::try_from(item.range().start).ok())
        .filter(|start| {
            *start > 0
                && source
                    .text
                    .get(..*start)
                    .is_some_and(|before| before.ends_with('\n'))
        })
        .collect();
    boundaries.dedup();
    let mut items = Vec::new();
    let mut start = 0;
    for boundary in boundaries.into_iter().chain([source.text.len()]) {
        if let Some(item) = source.text.get(start..boundary)
            && !item.is_empty()
        {
            items.push(item.to_string());
        }
        start = boundary;
    }
    items
}

pub fn joined(items: &[String]) -> String {
    let mut text = String::new();
    for item in items {
        text.push_str(item);
        if !item.ends_with('\n') {
            text.push('\n');
        }
    }
    text
}
