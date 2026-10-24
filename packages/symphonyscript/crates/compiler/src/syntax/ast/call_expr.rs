use super::arg_list::ArgList;
use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CallExpr(SyntaxNode);

impl AstNode for CallExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::CallExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl CallExpr {
    pub fn callee(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn args(&self) -> Option<ArgList> {
        child(&self.0)
    }
}
