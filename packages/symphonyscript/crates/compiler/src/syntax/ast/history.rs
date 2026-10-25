use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{children, name_at};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct History<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for History<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::History).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> History<'p> {
    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn offset(&self) -> Option<Expr<'p>> {
        children(&self.0).next()
    }

    pub fn fallback(&self) -> Option<Expr<'p>> {
        children(&self.0).nth(1)
    }
}
