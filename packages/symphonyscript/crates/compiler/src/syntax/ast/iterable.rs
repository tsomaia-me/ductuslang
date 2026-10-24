use super::ast_node::AstNode;
use super::expr::Expr;
use super::range_expr::RangeExpr;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Iterable {
    Range(RangeExpr),
    Expr(Expr),
}

impl AstNode for Iterable {
    fn cast(node: SyntaxNode) -> Option<Self> {
        match node.kind() {
            SyntaxKind::RangeExpr => RangeExpr::cast(node).map(Iterable::Range),
            _ => Expr::cast(node).map(Iterable::Expr),
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        match self {
            Iterable::Range(node) => node.syntax(),
            Iterable::Expr(node) => node.syntax(),
        }
    }
}
