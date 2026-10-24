use super::ast_node::AstNode;
use super::placement::Placement;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParenPlacement<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for ParenPlacement<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::ParenPlacement).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> ParenPlacement<'p> {
    pub fn placement(&self) -> Option<Placement<'p>> {
        child(&self.0)
    }
}
