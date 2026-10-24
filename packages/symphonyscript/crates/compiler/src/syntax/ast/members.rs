use super::ast_node::AstNode;
use super::member::Member;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Members<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Members<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Members).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Members<'p> {
    pub fn members(&self) -> impl Iterator<Item = Member<'p>> + use<'p> {
        children(&self.0)
    }
}
