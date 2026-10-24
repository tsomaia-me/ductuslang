use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at, token};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AttrEntry<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for AttrEntry<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::AttrEntry).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> AttrEntry<'p> {
    pub fn is_negated(&self) -> bool {
        token(&self.0, SyntaxKind::Bang).is_some()
    }

    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
