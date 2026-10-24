use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FieldExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for FieldExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::FieldExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> FieldExpr<'p> {
    pub fn receiver(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn field(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }
}
