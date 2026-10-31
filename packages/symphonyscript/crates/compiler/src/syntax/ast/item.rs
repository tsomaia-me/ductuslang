use super::ast_node::AstNode;
use super::cell_decl::CellDecl;
use super::fn_decl::FnDecl;
use super::input_decl::InputDecl;
use super::let_decl::LetDecl;
use super::pattern_decl::PatternDecl;
use super::placement::Placement;
use super::wire_placement::WirePlacement;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Item<'p> {
    Input(InputDecl<'p>),
    Let(LetDecl<'p>),
    Cell(CellDecl<'p>),
    Fn(FnDecl<'p>),
    Pattern(PatternDecl<'p>),
    Placement(Placement<'p>),
    Wire(WirePlacement<'p>),
}

impl<'p> AstNode<'p> for Item<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        match node.kind() {
            SyntaxKind::InputDecl => InputDecl::cast(node).map(Item::Input),
            SyntaxKind::LetDecl => LetDecl::cast(node).map(Item::Let),
            SyntaxKind::CellDecl => CellDecl::cast(node).map(Item::Cell),
            SyntaxKind::FnDecl => FnDecl::cast(node).map(Item::Fn),
            SyntaxKind::PatternDecl => PatternDecl::cast(node).map(Item::Pattern),
            SyntaxKind::Placement => Placement::cast(node).map(Item::Placement),
            SyntaxKind::WirePlacement => WirePlacement::cast(node).map(Item::Wire),
            _ => None,
        }
    }

    fn syntax(&self) -> &NodeView<'p> {
        match self {
            Item::Input(node) => node.syntax(),
            Item::Let(node) => node.syntax(),
            Item::Cell(node) => node.syntax(),
            Item::Fn(node) => node.syntax(),
            Item::Pattern(node) => node.syntax(),
            Item::Placement(node) => node.syntax(),
            Item::Wire(node) => node.syntax(),
        }
    }
}
