use super::ast_node::AstNode;
use super::iterable::Iterable;
use super::member::Member;
use super::name::Name;
use super::support::{child, children, name_at};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BundleFor(SyntaxNode);

impl AstNode for BundleFor {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::BundleFor).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl BundleFor {
    pub fn variable(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn iterable(&self) -> Option<Iterable> {
        child(&self.0)
    }

    pub fn members(&self) -> impl Iterator<Item = Member> + use<> {
        children(&self.0)
    }
}
