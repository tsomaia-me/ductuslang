use super::ast_node::AstNode;
use super::item::Item;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct File<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for File<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::File).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> File<'p> {
    pub fn items(&self) -> impl Iterator<Item = Item<'p>> + use<'p> {
        children(&self.0)
    }
}
