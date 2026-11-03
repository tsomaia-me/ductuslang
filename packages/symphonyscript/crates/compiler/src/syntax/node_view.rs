use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Range;

use super::children::{Children, Elements};
use super::descendants::Descendants;
use super::element_view::ElementView;
use super::parsed::{NO_NODE, Parsed};
use super::preorder::Preorder;
use super::syntax_kind::SyntaxKind;
use super::token_view::TokenView;
use super::walk::Walk;

#[derive(Clone, Copy)]
pub struct NodeView<'p> {
    parsed: &'p Parsed,
    index: u32,
}

impl<'p> NodeView<'p> {
    pub(crate) fn new(parsed: &'p Parsed, index: u32) -> Self {
        Self { parsed, index }
    }

    pub(crate) fn index(&self) -> u32 {
        self.index
    }

    pub fn kind(&self) -> SyntaxKind {
        self.parsed
            .tree()
            .kinds
            .get(self.index as usize)
            .copied()
            .unwrap_or(SyntaxKind::ErrorNode)
    }

    pub fn range(&self) -> Range<u32> {
        let (first, end) = self.parsed.tree().tokens(self.index);
        self.parsed.token_start(first)..self.parsed.token_start(end)
    }

    pub fn text(&self) -> String {
        self.slice().to_string()
    }

    pub fn parent(&self) -> Option<NodeView<'p>> {
        let parent = self.parsed.parent_of(self.index);
        (parent != NO_NODE).then(|| NodeView::new(self.parsed, parent))
    }

    pub fn ancestors(&self) -> impl Iterator<Item = NodeView<'p>> + use<'p> {
        std::iter::successors(Some(*self), NodeView::parent)
    }

    pub fn children(&self) -> impl Iterator<Item = NodeView<'p>> + use<'p> {
        let parsed = self.parsed;
        Children::of(parsed, self.index).map(move |child| {
            crate::semantics::counted_walk::CountedWalk::SyntaxViewElement.step();
            NodeView::new(parsed, child)
        })
    }

    pub fn children_with_tokens(&self) -> impl Iterator<Item = ElementView<'p>> + use<'p> {
        Elements::of(self.parsed, self.index).inspect(|_| {
            crate::semantics::counted_walk::CountedWalk::SyntaxViewElement.step();
        })
    }

    pub(crate) fn descendants(&self) -> impl Iterator<Item = NodeView<'p>> + use<'p> {
        let parsed = self.parsed;
        Descendants::of(parsed, self.index).map(move |node| {
            crate::semantics::counted_walk::CountedWalk::SyntaxViewElement.step();
            NodeView::new(parsed, node)
        })
    }

    pub fn preorder(&self) -> impl Iterator<Item = Walk<'p>> + use<'p> {
        Preorder::of(self.parsed, self.index).inspect(|_| {
            crate::semantics::counted_walk::CountedWalk::SyntaxViewElement.step();
        })
    }

    pub fn first_token(&self) -> Option<TokenView<'p>> {
        let (first, end) = self.parsed.tree().tokens(self.index);
        (first < end).then(|| TokenView::new(self.parsed, first))
    }

    pub fn last_token(&self) -> Option<TokenView<'p>> {
        let (first, end) = self.parsed.tree().tokens(self.index);
        (first < end).then(|| TokenView::new(self.parsed, end - 1))
    }

    pub fn token_at_offset(&self, offset: u32) -> Vec<TokenView<'p>> {
        let range = self.range();
        if offset < range.start || offset > range.end {
            return Vec::new();
        }
        let (first, end) = self.parsed.tree().tokens(self.index);
        let starts = self
            .parsed
            .tokens()
            .starts
            .get(first as usize..end as usize)
            .unwrap_or_default();
        let mut at =
            first + u32::try_from(starts.partition_point(|start| *start <= offset)).unwrap_or(0);
        let mut found = Vec::new();
        while at > first {
            at -= 1;
            let start = self.parsed.token_start(at);
            let token_end = self.parsed.token_start(at + 1);
            if token_end < offset {
                break;
            }
            if start < token_end {
                found.push(TokenView::new(self.parsed, at));
            }
        }
        found.reverse();
        found
    }

    fn slice(&self) -> &'p str {
        let range = self.range();
        self.parsed
            .text()
            .get(range.start as usize..range.end as usize)
            .unwrap_or_default()
    }
}

impl PartialEq for NodeView<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.parsed, other.parsed) && self.index == other.index
    }
}

impl Eq for NodeView<'_> {}

impl Hash for NodeView<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.parsed, state);
        self.index.hash(state);
    }
}

impl fmt::Debug for NodeView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{:?}", self.kind(), self.range())
    }
}

impl fmt::Display for NodeView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slice())
    }
}
