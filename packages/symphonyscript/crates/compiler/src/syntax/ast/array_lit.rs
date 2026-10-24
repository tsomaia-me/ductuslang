use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::children;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArrayLit<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for ArrayLit<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::ArrayLit).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> ArrayLit<'p> {
    pub fn elements(&self) -> impl Iterator<Item = Expr<'p>> + use<'p> {
        children(&self.0)
    }
}
