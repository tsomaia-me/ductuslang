use super::ast_node::AstNode;
use super::cell_decl::CellDecl;
use super::fn_decl::FnDecl;
use super::let_decl::LetDecl;
use super::package_decl::PackageDecl;
use super::pattern_decl::PatternDecl;
use super::placement::Placement;
use super::publish::Publish;
use super::require::Require;
use super::type_decl::TypeDecl;
use super::r#use::Use;
use super::wire_placement::WirePlacement;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Item<'p> {
    Let(LetDecl<'p>),
    Cell(CellDecl<'p>),
    Fn(FnDecl<'p>),
    Pattern(PatternDecl<'p>),
    Type(TypeDecl<'p>),
    Placement(Placement<'p>),
    Wire(WirePlacement<'p>),
    Package(PackageDecl<'p>),
    Require(Require<'p>),
    Use(Use<'p>),
    Publish(Publish<'p>),
}

impl<'p> AstNode<'p> for Item<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        match node.kind() {
            SyntaxKind::LetDecl => LetDecl::cast(node).map(Item::Let),
            SyntaxKind::CellDecl => CellDecl::cast(node).map(Item::Cell),
            SyntaxKind::FnDecl => FnDecl::cast(node).map(Item::Fn),
            SyntaxKind::PatternDecl => PatternDecl::cast(node).map(Item::Pattern),
            SyntaxKind::TypeDecl => TypeDecl::cast(node).map(Item::Type),
            SyntaxKind::Placement => Placement::cast(node).map(Item::Placement),
            SyntaxKind::WirePlacement => WirePlacement::cast(node).map(Item::Wire),
            SyntaxKind::PackageDecl => PackageDecl::cast(node).map(Item::Package),
            SyntaxKind::Require => Require::cast(node).map(Item::Require),
            SyntaxKind::Use => Use::cast(node).map(Item::Use),
            SyntaxKind::Publish => Publish::cast(node).map(Item::Publish),
            _ => None,
        }
    }

    fn syntax(&self) -> &NodeView<'p> {
        match self {
            Item::Let(node) => node.syntax(),
            Item::Cell(node) => node.syntax(),
            Item::Fn(node) => node.syntax(),
            Item::Pattern(node) => node.syntax(),
            Item::Type(node) => node.syntax(),
            Item::Placement(node) => node.syntax(),
            Item::Wire(node) => node.syntax(),
            Item::Package(node) => node.syntax(),
            Item::Require(node) => node.syntax(),
            Item::Use(node) => node.syntax(),
            Item::Publish(node) => node.syntax(),
        }
    }
}
