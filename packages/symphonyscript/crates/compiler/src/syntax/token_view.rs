use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Range;

use super::node_view::NodeView;
use super::parsed::{NO_NODE, Parsed};
use super::syntax_kind::SyntaxKind;

#[derive(Clone, Copy)]
pub struct TokenView<'p> {
    parsed: &'p Parsed,
    index: u32,
}

impl<'p> TokenView<'p> {
    pub(crate) fn new(parsed: &'p Parsed, index: u32) -> Self {
        Self { parsed, index }
    }

    pub(crate) fn index(&self) -> u32 {
        self.index
    }

    pub fn kind(&self) -> SyntaxKind {
        self.parsed
            .tokens()
            .kind(self.index as usize)
            .unwrap_or(SyntaxKind::Error)
    }

    pub fn text(&self) -> &str {
        let range = self.range();
        self.parsed
            .text()
            .get(range.start as usize..range.end as usize)
            .unwrap_or_default()
    }

    pub fn range(&self) -> Range<u32> {
        self.parsed.token_start(self.index)..self.parsed.token_start(self.index + 1)
    }

    pub fn parent(&self) -> Option<NodeView<'p>> {
        let parent = self.parsed.parent_of_token(self.index);
        (parent != NO_NODE).then(|| NodeView::new(self.parsed, parent))
    }

    pub fn next_token(&self) -> Option<TokenView<'p>> {
        let next = self.index + 1;
        (next < self.parsed.token_count()).then(|| TokenView::new(self.parsed, next))
    }

    pub fn prev_token(&self) -> Option<TokenView<'p>> {
        self.index
            .checked_sub(1)
            .map(|previous| TokenView::new(self.parsed, previous))
    }
}

impl PartialEq for TokenView<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.parsed, other.parsed) && self.index == other.index
    }
}

impl Eq for TokenView<'_> {}

impl Hash for TokenView<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.parsed, state);
        self.index.hash(state);
    }
}

impl fmt::Debug for TokenView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{:?} {:?}", self.kind(), self.range(), self.text())
    }
}

impl fmt::Display for TokenView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}
