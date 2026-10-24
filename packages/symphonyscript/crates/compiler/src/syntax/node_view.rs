use std::fmt;
use std::marker::PhantomData;
use std::ops::Range;

use rowan::{NodeOrToken, TextSize};

use super::element_view::ElementView;
use super::navigation;
use super::parsed::Parsed;
use super::syntax_kind::SyntaxKind;
use super::syntax_node::SyntaxNode;
use super::token_view::TokenView;
use super::walk::Walk;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NodeView<'p> {
    raw: SyntaxNode,
    parsed: PhantomData<&'p Parsed>,
}

impl<'p> NodeView<'p> {
    pub(crate) fn new(raw: SyntaxNode) -> Self {
        Self {
            raw,
            parsed: PhantomData,
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.raw.kind()
    }

    pub fn range(&self) -> Range<u32> {
        let range = self.raw.text_range();
        u32::from(range.start())..u32::from(range.end())
    }

    pub fn text(&self) -> String {
        self.raw.to_string()
    }

    pub fn parent(&self) -> Option<NodeView<'p>> {
        self.raw.parent().map(NodeView::new)
    }

    pub fn ancestors(&self) -> impl Iterator<Item = NodeView<'p>> + use<'p> {
        self.raw.ancestors().map(NodeView::new)
    }

    pub fn children(&self) -> impl Iterator<Item = NodeView<'p>> + use<'p> {
        self.raw.children().map(NodeView::new)
    }

    pub fn children_with_tokens(&self) -> impl Iterator<Item = ElementView<'p>> + use<'p> {
        self.raw.children_with_tokens().map(ElementView::new)
    }

    pub fn preorder(&self) -> impl Iterator<Item = Walk<'p>> + use<'p> {
        self.raw.preorder_with_tokens().map(|event| match event {
            rowan::WalkEvent::Enter(element) => Walk::Enter(ElementView::new(element)),
            rowan::WalkEvent::Leave(element) => Walk::Leave(ElementView::new(element)),
        })
    }

    pub fn first_token(&self) -> Option<TokenView<'p>> {
        navigation::first_token(NodeOrToken::Node(self.raw.clone())).map(TokenView::new)
    }

    pub fn last_token(&self) -> Option<TokenView<'p>> {
        navigation::last_token(NodeOrToken::Node(self.raw.clone())).map(TokenView::new)
    }

    pub fn token_at_offset(&self, offset: u32) -> Vec<TokenView<'p>> {
        let range = self.raw.text_range();
        let offset = TextSize::from(offset);
        if offset < range.start() || offset > range.end() {
            return Vec::new();
        }
        navigation::tokens_at_offset(&self.raw, offset)
            .into_iter()
            .map(TokenView::new)
            .collect()
    }
}

impl fmt::Debug for NodeView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{:?}", self.kind(), self.range())
    }
}

impl fmt::Display for NodeView<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.raw, f)
    }
}
