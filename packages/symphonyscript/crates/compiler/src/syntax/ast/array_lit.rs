use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArrayLit(SyntaxNode);

impl AstNode for ArrayLit {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::ArrayLit).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ArrayLit {
    pub fn elements(&self) -> impl Iterator<Item = Expr> + use<> {
        children(&self.0)
    }
}
