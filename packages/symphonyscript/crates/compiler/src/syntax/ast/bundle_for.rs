use super::ast_node::AstNode;
use super::loop_head::LoopHead;
use super::member::Member;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BundleFor<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for BundleFor<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::BundleFor).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> BundleFor<'p> {
    pub fn head(&self) -> LoopHead<'p> {
        LoopHead::of(&self.0)
    }

    pub fn members(&self) -> impl Iterator<Item = Member<'p>> + use<'p> {
        children(&self.0)
    }
}
