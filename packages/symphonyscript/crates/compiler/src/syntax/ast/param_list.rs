use super::ast_node::AstNode;
use super::param::Param;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParamList<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for ParamList<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::ParamList).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> ParamList<'p> {
    pub fn params(&self) -> impl Iterator<Item = Param<'p>> + use<'p> {
        children(&self.0)
    }
}
