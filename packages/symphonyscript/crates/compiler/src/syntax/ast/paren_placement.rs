use super::ast_node::AstNode;
use super::placement::Placement;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParenPlacement(SyntaxNode);

impl AstNode for ParenPlacement {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::ParenPlacement).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ParenPlacement {
    pub fn placement(&self) -> Option<Placement> {
        child(&self.0)
    }
}
