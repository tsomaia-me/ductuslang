use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MatchArm(SyntaxNode);

impl AstNode for MatchArm {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::MatchArm).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MatchArm {
    pub fn label(&self) -> Option<Expr> {
        children::<Expr>(&self.0).next()
    }

    pub fn value(&self) -> Option<Expr> {
        children::<Expr>(&self.0).nth(1)
    }
}
