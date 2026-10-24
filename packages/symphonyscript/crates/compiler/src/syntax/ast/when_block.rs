use super::ast_node::AstNode;
use super::block::Block;
use super::expr::Expr;
use super::guard_arm::GuardArm;
use super::support::{child, children};
use super::when_else::WhenElse;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct WhenBlock<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for WhenBlock<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::WhenBlock).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> WhenBlock<'p> {
    pub fn condition(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn block(&self) -> Option<Block<'p>> {
        child(&self.0)
    }

    pub fn guard_arms(&self) -> impl Iterator<Item = GuardArm<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn else_branch(&self) -> Option<WhenElse<'p>> {
        child(&self.0)
    }
}
