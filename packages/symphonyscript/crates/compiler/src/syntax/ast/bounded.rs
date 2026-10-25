use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{child, token};
use crate::syntax::{NodeView, SyntaxKind, TokenView};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Bounded<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Bounded<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Bounded).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Bounded<'p> {
    pub fn bound(&self) -> Option<TokenView<'p>> {
        token(&self.0, SyntaxKind::Number)
    }

    pub fn fallback(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
