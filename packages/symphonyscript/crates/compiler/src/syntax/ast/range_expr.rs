use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{expr_after, expr_before};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RangeExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for RangeExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::RangeExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> RangeExpr<'p> {
    pub fn start(&self) -> Option<Expr<'p>> {
        expr_before(&self.0, |kind| kind == SyntaxKind::Dot2)
    }

    pub fn end(&self) -> Option<Expr<'p>> {
        expr_after(&self.0, |kind| kind == SyntaxKind::Dot2, |_| false)
    }
}
