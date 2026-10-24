use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParenExpr(SyntaxNode);

impl AstNode for ParenExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::ParenExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ParenExpr {
    pub fn inner(&self) -> Option<Expr> {
        child(&self.0)
    }
}
