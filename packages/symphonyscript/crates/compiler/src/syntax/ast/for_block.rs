use super::ast_node::AstNode;
use super::block::Block;
use super::loop_head::LoopHead;
use super::support::child;
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
    pub fn head(&self) -> LoopHead<'p> {
        LoopHead::of(&self.0)
    }

    pub fn block(&self) -> Option<Block<'p>> {
        child(&self.0)
    }
}
