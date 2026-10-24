use super::ast_node::AstNode;
use super::member::Member;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Members(SyntaxNode);

impl AstNode for Members {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Members).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Members {
    pub fn members(&self) -> impl Iterator<Item = Member> + use<> {
        children(&self.0)
    }
}
