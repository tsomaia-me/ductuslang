use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::children;
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
        children::<Expr>(&self.0).next()
    }

    pub fn index(&self) -> Option<Expr> {
        children::<Expr>(&self.0).nth(1)
    }
}
