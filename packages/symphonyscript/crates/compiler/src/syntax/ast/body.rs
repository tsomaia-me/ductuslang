use super::ast_node::AstNode;
use super::block::Block;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Body(SyntaxNode);

impl AstNode for Body {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Body).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Body {
    pub fn block(&self) -> Option<Block> {
        child(&self.0)
    }
}
