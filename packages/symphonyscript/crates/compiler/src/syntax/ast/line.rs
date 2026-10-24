use super::ast_node::AstNode;
use super::member::Member;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Line<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Line<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Line).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Line<'p> {
    pub fn members(&self) -> impl Iterator<Item = Member<'p>> + use<'p> {
        children(&self.0)
    }
}
