use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DefaultArg(SyntaxNode);

impl AstNode for DefaultArg {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::DefaultArg).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl DefaultArg {
    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }
}
