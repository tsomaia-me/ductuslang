use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind, TokenView};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PrefixExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for PrefixExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::PrefixExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> PrefixExpr<'p> {
    pub fn op(&self) -> Option<TokenView<'p>> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| !token.kind().is_trivia())
    }

    pub fn operand(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }
}
