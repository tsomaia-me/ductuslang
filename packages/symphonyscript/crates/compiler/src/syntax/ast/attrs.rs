use super::ast_node::AstNode;
use super::attr_entry::AttrEntry;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Attrs(SyntaxNode);

impl AstNode for Attrs {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Attrs).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Attrs {
    pub fn entries(&self) -> impl Iterator<Item = AttrEntry> + use<> {
        children(&self.0)
    }
}
