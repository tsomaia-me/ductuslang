use std::ops::Range;

use super::node_view::NodeView;
use super::syntax_element::SyntaxElement;
use super::syntax_kind::SyntaxKind;
use super::token_view::TokenView;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ElementView<'p> {
    Node(NodeView<'p>),
    Token(TokenView<'p>),
}

impl<'p> ElementView<'p> {
    pub(crate) fn new(raw: SyntaxElement) -> Self {
        match raw {
            rowan::NodeOrToken::Node(node) => ElementView::Node(NodeView::new(node)),
            rowan::NodeOrToken::Token(token) => ElementView::Token(TokenView::new(token)),
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        match self {
            ElementView::Node(node) => node.kind(),
            ElementView::Token(token) => token.kind(),
        }
    }

    pub fn range(&self) -> Range<u32> {
        match self {
            ElementView::Node(node) => node.range(),
            ElementView::Token(token) => token.range(),
        }
    }

    pub fn as_node(&self) -> Option<&NodeView<'p>> {
        match self {
            ElementView::Node(node) => Some(node),
            ElementView::Token(_) => None,
        }
    }

    pub fn as_token(&self) -> Option<&TokenView<'p>> {
        match self {
            ElementView::Node(_) => None,
            ElementView::Token(token) => Some(token),
        }
    }

    pub fn into_node(self) -> Option<NodeView<'p>> {
        match self {
            ElementView::Node(node) => Some(node),
            ElementView::Token(_) => None,
        }
    }

    pub fn into_token(self) -> Option<TokenView<'p>> {
        match self {
            ElementView::Node(_) => None,
            ElementView::Token(token) => Some(token),
        }
    }
}
