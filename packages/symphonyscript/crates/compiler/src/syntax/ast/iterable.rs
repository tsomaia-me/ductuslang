use super::ast_node::AstNode;
use super::expr::Expr;
use super::range_expr::RangeExpr;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Iterable<'p> {
    Range(RangeExpr<'p>),
    Expr(Expr<'p>),
}

impl<'p> AstNode<'p> for Iterable<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        match node.kind() {
            SyntaxKind::RangeExpr => RangeExpr::cast(node).map(Iterable::Range),
            _ => Expr::cast(node).map(Iterable::Expr),
        }
    }

    fn syntax(&self) -> &NodeView<'p> {
        match self {
            Iterable::Range(node) => node.syntax(),
            Iterable::Expr(node) => node.syntax(),
        }
    }
}
