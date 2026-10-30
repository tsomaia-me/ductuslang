use super::ast_node::AstNode;
use super::expr::Expr;
use super::for_expr_head::ForExprHead;
use super::local_let::LocalLet;
use super::support::{child, children};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ForExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for ForExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        matches!(node.kind(), SyntaxKind::ForExpr | SyntaxKind::ForExprBlock).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> ForExpr<'p> {
    pub fn head(&self) -> Option<ForExprHead<'p>> {
        child(&self.0)
    }

    pub fn is_block(&self) -> bool {
        self.0.kind() == SyntaxKind::ForExprBlock
    }

    pub fn lets(&self) -> impl Iterator<Item = LocalLet<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn body(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
