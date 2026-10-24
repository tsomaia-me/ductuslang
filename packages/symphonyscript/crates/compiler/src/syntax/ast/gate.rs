use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Gate<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Gate<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Gate).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Gate<'p> {
    pub fn condition(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
