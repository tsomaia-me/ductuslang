use super::ast_node::AstNode;
use super::attr_entry::AttrEntry;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Attrs<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Attrs<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Attrs).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Attrs<'p> {
    pub fn entries(&self) -> impl Iterator<Item = AttrEntry<'p>> + use<'p> {
        children(&self.0)
    }
}
