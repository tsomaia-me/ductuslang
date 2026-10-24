use super::ast_node::AstNode;
use super::for_block::ForBlock;
use super::line::Line;
use super::placement::Placement;
use super::when_block::WhenBlock;
use super::wire_placement::WirePlacement;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum BodyItem {
    Placement(Placement),
    Wire(WirePlacement),
    Line(Line),
    For(ForBlock),
    When(WhenBlock),
}

impl AstNode for BodyItem {
    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::Placement => Placement::cast(node).map(BodyItem::Placement),
            SyntaxKind::WirePlacement => WirePlacement::cast(node).map(BodyItem::Wire),
            SyntaxKind::Line => Line::cast(node).map(BodyItem::Line),
            SyntaxKind::ForBlock => ForBlock::cast(node).map(BodyItem::For),
            SyntaxKind::WhenBlock => WhenBlock::cast(node).map(BodyItem::When),
            _ => None,
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            BodyItem::Placement(node) => node.syntax(),
            BodyItem::Wire(node) => node.syntax(),
            BodyItem::Line(node) => node.syntax(),
            BodyItem::For(node) => node.syntax(),
            BodyItem::When(node) => node.syntax(),
        }
    }
}
