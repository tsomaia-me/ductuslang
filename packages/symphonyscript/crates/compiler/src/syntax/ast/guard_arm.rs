use super::ast_node::AstNode;
use super::block::Block;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GuardArm<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for GuardArm<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::GuardArm).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> GuardArm<'p> {
    pub fn guard(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn block(&self) -> Option<Block<'p>> {
        child(&self.0)
    }
}
