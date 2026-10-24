use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::children;
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
        children::<Expr>(&self.0).next()
    }

    pub fn end(&self) -> Option<Expr> {
        children::<Expr>(&self.0).nth(1)
    }
}
