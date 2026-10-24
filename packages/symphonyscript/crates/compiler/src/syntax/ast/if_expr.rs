use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::expr_after;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct IfExpr(SyntaxNode);

impl AstNode for IfExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::IfExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl IfExpr {
    pub fn condition(&self) -> Option<Expr> {
        expr_after(
            &self.0,
            |kind| kind == SyntaxKind::IfKw,
            |kind| kind == SyntaxKind::ThenKw || kind == SyntaxKind::ElseKw,
        )
    }

    pub fn then_branch(&self) -> Option<Expr> {
        expr_after(
            &self.0,
            |kind| kind == SyntaxKind::ThenKw,
            |kind| kind == SyntaxKind::ElseKw,
        )
    }

    pub fn else_branch(&self) -> Option<Expr> {
        expr_after(&self.0, |kind| kind == SyntaxKind::ElseKw, |_| false)
    }
}
