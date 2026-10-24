use super::ast_node::AstNode;
use super::block::Block;
use super::iterable::Iterable;
use super::name::Name;
use super::support::{child, name_at};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ForBlock(SyntaxNode);

impl AstNode for ForBlock {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::ForBlock).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ForBlock {
    pub fn variable(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn iterable(&self) -> Option<Iterable> {
        child(&self.0)
    }

    pub fn block(&self) -> Option<Block> {
        child(&self.0)
    }
}
