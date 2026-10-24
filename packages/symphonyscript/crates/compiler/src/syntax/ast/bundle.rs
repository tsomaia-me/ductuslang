use super::ast_node::AstNode;
use super::bundle_for::BundleFor;
use super::member::Member;
use super::support::{child, children};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Bundle<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Bundle<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Bundle).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Bundle<'p> {
    pub fn members(&self) -> impl Iterator<Item = Member<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn bundle_for(&self) -> Option<BundleFor<'p>> {
        child(&self.0)
    }
}
