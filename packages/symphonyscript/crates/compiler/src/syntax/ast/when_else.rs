use super::ast_node::AstNode;
use super::block::Block;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct WhenElse<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for WhenElse<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::WhenElse).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> WhenElse<'p> {
    pub fn block(&self) -> Option<Block<'p>> {
        child(&self.0)
    }
}
