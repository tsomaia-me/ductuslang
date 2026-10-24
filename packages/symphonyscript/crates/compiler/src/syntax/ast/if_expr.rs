use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::children;
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
        children::<Expr>(&self.0).next()
    }

    pub fn then_branch(&self) -> Option<Expr> {
        children::<Expr>(&self.0).nth(1)
    }

    pub fn else_branch(&self) -> Option<Expr> {
        children::<Expr>(&self.0).nth(2)
    }
}
