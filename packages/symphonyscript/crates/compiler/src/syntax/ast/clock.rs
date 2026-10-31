use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{children, token};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Clock<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Clock<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Clock).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Clock<'p> {
    pub fn is_list(&self) -> bool {
        token(&self.0, SyntaxKind::LParen).is_some()
    }

    pub fn entries(&self) -> impl Iterator<Item = Expr<'p>> + use<'p> {
        children(&self.0)
    }
}
