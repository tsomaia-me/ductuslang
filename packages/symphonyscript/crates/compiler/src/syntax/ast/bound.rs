use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{child, token};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Bound<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Bound<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Bound).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Bound<'p> {
    pub fn from_end(&self) -> bool {
        token(&self.0, SyntaxKind::Caret).is_some()
    }

    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
