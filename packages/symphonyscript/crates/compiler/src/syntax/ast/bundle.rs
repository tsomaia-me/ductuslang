use super::ast_node::AstNode;
use super::bundle_for::BundleFor;
use super::member::Member;
use super::support::{child, children};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Bundle(SyntaxNode);

impl AstNode for Bundle {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Bundle).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Bundle {
    pub fn members(&self) -> impl Iterator<Item = Member> + use<> {
        children(&self.0)
    }

    pub fn bundle_for(&self) -> Option<BundleFor> {
        child(&self.0)
    }
}
