use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ElseArm(SyntaxNode);

impl AstNode for ElseArm {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::ElseArm).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ElseArm {
    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }
}
