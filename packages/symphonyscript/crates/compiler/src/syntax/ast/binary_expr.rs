use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{expr_after, expr_before, token};
use crate::syntax::{NodeView, SyntaxKind, TokenView};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BinaryExpr<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for BinaryExpr<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::BinaryExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> BinaryExpr<'p> {
    pub fn lhs(&self) -> Option<Expr<'p>> {
        expr_before(&self.0, |kind| !kind.is_trivia())
    }

    pub fn rhs(&self) -> Option<Expr<'p>> {
        expr_after(&self.0, |kind| !kind.is_trivia(), |_| false)
    }

    pub fn op(&self) -> Option<TokenView<'p>> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| !token.kind().is_trivia())
    }

    pub fn is_not(&self) -> bool {
        token(&self.0, SyntaxKind::IsKw).is_some() && token(&self.0, SyntaxKind::NotKw).is_some()
    }
}
