use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{expr_after, expr_before};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MatchArm<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for MatchArm<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::MatchArm).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> MatchArm<'p> {
    pub fn label(&self) -> Option<Expr<'p>> {
        expr_before(&self.0, |kind| kind == SyntaxKind::Colon)
    }

    pub fn value(&self) -> Option<Expr<'p>> {
        expr_after(&self.0, |kind| kind == SyntaxKind::Colon, |_| false)
    }
}
