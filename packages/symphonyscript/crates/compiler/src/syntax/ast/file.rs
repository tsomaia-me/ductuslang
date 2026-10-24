use super::ast_node::AstNode;
use super::item::Item;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct File(SyntaxNode);

impl AstNode for File {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::File).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl File {
    pub fn items(&self) -> impl Iterator<Item = Item> + use<> {
        children(&self.0)
    }
}
