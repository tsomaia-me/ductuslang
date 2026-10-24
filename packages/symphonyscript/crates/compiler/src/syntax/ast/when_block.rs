use super::ast_node::AstNode;
use super::block::Block;
use super::expr::Expr;
use super::guard_arm::GuardArm;
use super::support::{child, children};
use super::when_else::WhenElse;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct WhenBlock(SyntaxNode);

impl AstNode for WhenBlock {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::WhenBlock).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl WhenBlock {
    pub fn condition(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn block(&self) -> Option<Block> {
        child(&self.0)
    }

    pub fn guard_arms(&self) -> impl Iterator<Item = GuardArm> + use<> {
        children(&self.0)
    }

    pub fn else_branch(&self) -> Option<WhenElse> {
        child(&self.0)
    }
}
