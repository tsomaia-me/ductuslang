use symphonyscript::Source;
use symphonyscript::syntax::{ElementView, Walk, parse};
use symphonyscript_core::Vocabulary;

use crate::program::Program;

pub fn syntax_nodes(program: &Program, vocabulary: &Vocabulary) -> u64 {
    program
        .sources
        .iter()
        .map(|source| {
            let parsed = parse(
                &Source {
                    path: source.path.clone(),
                    text: source.text.clone(),
                },
                vocabulary,
            );
            let entered = parsed
                .root()
                .preorder()
                .filter(|event| matches!(event, Walk::Enter(ElementView::Node(_))))
                .count();
            u64::try_from(entered).unwrap_or(u64::MAX)
        })
        .sum()
}
