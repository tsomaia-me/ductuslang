use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{expr_after, expr_before};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct IndexExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for IndexExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::IndexExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> IndexExpr<'p> {
    pub fn base(&self) -> Option<Expr<'p>> {
        expr_before(&self.0, |kind| kind == SyntaxKind::LBracket)
    }

    pub fn index(&self) -> Option<Expr<'p>> {
        expr_after(
            &self.0,
            |kind| kind == SyntaxKind::LBracket,
            |kind| kind == SyntaxKind::RBracket,
        )
    }
}
