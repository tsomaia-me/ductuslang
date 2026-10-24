use super::arg_list::ArgList;
use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CallExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for CallExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::CallExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> CallExpr<'p> {
    pub fn callee(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn args(&self) -> Option<ArgList<'p>> {
        child(&self.0)
    }
}
