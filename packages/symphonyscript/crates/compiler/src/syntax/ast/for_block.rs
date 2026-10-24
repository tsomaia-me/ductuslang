use super::ast_node::AstNode;
use super::block::Block;
use super::iterable::Iterable;
use super::name::Name;
use super::support::{child, name_at};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ForBlock<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for ForBlock<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::ForBlock).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> ForBlock<'p> {
    pub fn variable(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn iterable(&self) -> Option<Iterable<'p>> {
        child(&self.0)
    }

    pub fn block(&self) -> Option<Block<'p>> {
        child(&self.0)
    }
}
