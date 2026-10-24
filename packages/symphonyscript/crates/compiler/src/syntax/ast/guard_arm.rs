use super::ast_node::AstNode;
use super::block::Block;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GuardArm(SyntaxNode);

impl AstNode for GuardArm {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::GuardArm).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl GuardArm {
    pub fn guard(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn block(&self) -> Option<Block> {
        child(&self.0)
    }
}
