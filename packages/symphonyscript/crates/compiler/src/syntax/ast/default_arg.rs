use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DefaultArg<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for DefaultArg<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::DefaultArg).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> DefaultArg<'p> {
    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
