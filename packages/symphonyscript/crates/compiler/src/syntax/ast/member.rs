use super::ast_node::AstNode;
use super::bundle::Bundle;
use super::paren_placement::ParenPlacement;
use super::placement::Placement;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Member {
    Placement(Placement),
    Paren(ParenPlacement),
    Bundle(Bundle),
}

impl AstNode for Member {
    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Placement => Placement::cast(node).map(Member::Placement),
            SyntaxKind::ParenPlacement => ParenPlacement::cast(node).map(Member::Paren),
            SyntaxKind::Bundle => Bundle::cast(node).map(Member::Bundle),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Member::Placement(node) => node.syntax(),
            Member::Paren(node) => node.syntax(),
            Member::Bundle(node) => node.syntax(),
        }
    }
}
