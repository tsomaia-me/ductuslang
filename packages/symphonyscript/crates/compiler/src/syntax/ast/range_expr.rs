use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{expr_after, expr_before};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RangeExpr(SyntaxNode);

impl AstNode for RangeExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::RangeExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl RangeExpr {
    pub fn start(&self) -> Option<Expr> {
        expr_before(&self.0, |kind| kind == SyntaxKind::Dot2)
    }

    pub fn end(&self) -> Option<Expr> {
        expr_after(&self.0, |kind| kind == SyntaxKind::Dot2, |_| false)
    }
}
