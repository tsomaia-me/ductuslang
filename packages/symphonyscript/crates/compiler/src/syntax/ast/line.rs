use super::ast_node::AstNode;
use super::member::Member;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Line(SyntaxNode);

impl AstNode for Line {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Line).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Line {
    pub fn members(&self) -> impl Iterator<Item = Member> + use<> {
        children(&self.0)
    }
}
