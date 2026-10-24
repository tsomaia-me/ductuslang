use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{expr_after, expr_before};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct IndexExpr(SyntaxNode);

impl AstNode for IndexExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::IndexExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl IndexExpr {
    pub fn base(&self) -> Option<Expr> {
        expr_before(&self.0, |kind| kind == SyntaxKind::LBracket)
    }

    pub fn index(&self) -> Option<Expr> {
        expr_after(
            &self.0,
            |kind| kind == SyntaxKind::LBracket,
            |kind| kind == SyntaxKind::RBracket,
        )
    }
}
