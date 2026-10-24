use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at, token};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PatternParam<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for PatternParam<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::PatternParam).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> PatternParam<'p> {
    pub fn is_default(&self) -> bool {
        token(&self.0, SyntaxKind::DefaultKw).is_some()
    }

    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn default_value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
