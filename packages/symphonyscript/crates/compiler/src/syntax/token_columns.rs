use std::ops::Range;

use super::syntax_kind::SyntaxKind;
use super::token::Token;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct TokenColumns {
    pub(crate) kinds: Vec<SyntaxKind>,
    pub(crate) starts: Vec<u32>,
    pub(crate) sig: Vec<u32>,
}

impl TokenColumns {
    pub(crate) fn reserved(bytes: usize) -> Self {
        let tokens = bytes / 2;
        Self {
            kinds: Vec::with_capacity(tokens),
            starts: Vec::with_capacity(tokens + 1),
            sig: Vec::with_capacity(tokens),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.kinds.len()
    }

    pub(crate) fn kind(&self, index: usize) -> Option<SyntaxKind> {
        self.kinds.get(index).copied()
    }

    pub(crate) fn start(&self, index: usize) -> Option<u32> {
        self.starts.get(index).copied()
    }

    pub(crate) fn range(&self, index: usize) -> Option<Range<u32>> {
        let start = self.starts.get(index).copied()?;
        let end = self.starts.get(index + 1).copied()?;
        Some(start..end)
    }

    pub(crate) fn tokens(&self) -> Vec<Token> {
        self.kinds
            .iter()
            .enumerate()
            .filter_map(|(index, kind)| {
                Some(Token {
                    kind: *kind,
                    range: self.range(index)?,
                })
            })
            .collect()
    }
}
