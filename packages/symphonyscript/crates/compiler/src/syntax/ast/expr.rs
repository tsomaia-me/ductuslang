use super::array_lit::ArrayLit;
use super::ast_node::AstNode;
use super::binary_expr::BinaryExpr;
use super::call_expr::CallExpr;
use super::conversion::Conversion;
use super::field_expr::FieldExpr;
use super::if_expr::IfExpr;
use super::index_expr::IndexExpr;
use super::literal::Literal;
use super::name_ref::NameRef;
use super::paren_expr::ParenExpr;
use super::prefix_expr::PrefixExpr;
use super::string_lit::StringLit;
use super::to_expr::ToExpr;
use super::variant_ref::VariantRef;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Expr<'p> {
    If(IfExpr<'p>),
    Binary(BinaryExpr<'p>),
    Prefix(PrefixExpr<'p>),
    Paren(ParenExpr<'p>),
    Literal(Literal<'p>),
    String(StringLit<'p>),
    NameRef(NameRef<'p>),
    VariantRef(VariantRef<'p>),
    To(ToExpr<'p>),
    Conversion(Conversion<'p>),
    Array(ArrayLit<'p>),
    Field(FieldExpr<'p>),
    Index(IndexExpr<'p>),
    Call(CallExpr<'p>),
}

impl<'p> AstNode<'p> for Expr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        match node.kind() {
            SyntaxKind::IfExpr => IfExpr::cast(node).map(Expr::If),
            SyntaxKind::BinaryExpr => BinaryExpr::cast(node).map(Expr::Binary),
            SyntaxKind::PrefixExpr => PrefixExpr::cast(node).map(Expr::Prefix),
            SyntaxKind::ParenExpr => ParenExpr::cast(node).map(Expr::Paren),
            SyntaxKind::Literal => Literal::cast(node).map(Expr::Literal),
            SyntaxKind::StringLit => StringLit::cast(node).map(Expr::String),
            SyntaxKind::NameRef => NameRef::cast(node).map(Expr::NameRef),
            SyntaxKind::VariantRef => VariantRef::cast(node).map(Expr::VariantRef),
            SyntaxKind::ToExpr => ToExpr::cast(node).map(Expr::To),
            SyntaxKind::Conversion => Conversion::cast(node).map(Expr::Conversion),
            SyntaxKind::ArrayLit => ArrayLit::cast(node).map(Expr::Array),
            SyntaxKind::FieldExpr => FieldExpr::cast(node).map(Expr::Field),
            SyntaxKind::IndexExpr => IndexExpr::cast(node).map(Expr::Index),
            SyntaxKind::CallExpr => CallExpr::cast(node).map(Expr::Call),
            _ => None,
        }
    }

    fn syntax(&self) -> &NodeView<'p> {
        match self {
            Expr::If(node) => node.syntax(),
            Expr::Binary(node) => node.syntax(),
            Expr::Prefix(node) => node.syntax(),
            Expr::Paren(node) => node.syntax(),
            Expr::Literal(node) => node.syntax(),
            Expr::String(node) => node.syntax(),
            Expr::NameRef(node) => node.syntax(),
            Expr::VariantRef(node) => node.syntax(),
            Expr::To(node) => node.syntax(),
            Expr::Conversion(node) => node.syntax(),
            Expr::Array(node) => node.syntax(),
            Expr::Field(node) => node.syntax(),
            Expr::Index(node) => node.syntax(),
            Expr::Call(node) => node.syntax(),
        }
    }
}
