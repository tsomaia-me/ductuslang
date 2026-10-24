use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::expr_after;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct IfExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for IfExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::IfExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> IfExpr<'p> {
    pub fn condition(&self) -> Option<Expr<'p>> {
        expr_after(
            &self.0,
            |kind| kind == SyntaxKind::IfKw,
            |kind| kind == SyntaxKind::ThenKw || kind == SyntaxKind::ElseKw,
        )
    }

    pub fn then_branch(&self) -> Option<Expr<'p>> {
        expr_after(
            &self.0,
            |kind| kind == SyntaxKind::ThenKw,
            |kind| kind == SyntaxKind::ElseKw,
        )
    }

    pub fn else_branch(&self) -> Option<Expr<'p>> {
        expr_after(&self.0, |kind| kind == SyntaxKind::ElseKw, |_| false)
    }
}
