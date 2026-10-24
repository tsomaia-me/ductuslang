use super::ast_node::AstNode;
use super::bundle::Bundle;
use super::paren_placement::ParenPlacement;
use super::placement::Placement;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Member<'p> {
    Placement(Placement<'p>),
    Paren(ParenPlacement<'p>),
    Bundle(Bundle<'p>),
}

impl<'p> AstNode<'p> for Member<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Placement => Placement::cast(node).map(Member::Placement),
            SyntaxKind::ParenPlacement => ParenPlacement::cast(node).map(Member::Paren),
            SyntaxKind::Bundle => Bundle::cast(node).map(Member::Bundle),
            _ => None,
        }
    }

    fn syntax(&self) -> &NodeView<'p> {
        match self {
            Member::Placement(node) => node.syntax(),
            Member::Paren(node) => node.syntax(),
            Member::Bundle(node) => node.syntax(),
        }
    }
}
