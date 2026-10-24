use super::ast_node::AstNode;
use super::iterable::Iterable;
use super::member::Member;
use super::name::Name;
use super::support::{child, children, name_at};
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
    pub fn variable(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn iterable(&self) -> Option<Iterable<'p>> {
        child(&self.0)
    }

    pub fn members(&self) -> impl Iterator<Item = Member<'p>> + use<'p> {
        children(&self.0)
    }
}
