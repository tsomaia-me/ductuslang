use super::ast_node::AstNode;
use super::block::Block;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct WhenElse(SyntaxNode);

impl AstNode for WhenElse {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::WhenElse).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl WhenElse {
    pub fn block(&self) -> Option<Block> {
        child(&self.0)
    }
}
