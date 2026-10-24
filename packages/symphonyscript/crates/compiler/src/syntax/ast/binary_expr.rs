use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::{children, token};
use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct BinaryExpr(SyntaxNode);

impl AstNode for BinaryExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::BinaryExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl BinaryExpr {
    pub fn lhs(&self) -> Option<Expr> {
        children::<Expr>(&self.0).next()
    }

    pub fn rhs(&self) -> Option<Expr> {
        children::<Expr>(&self.0).nth(1)
    }

    pub fn op(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| !token.kind().is_trivia())
    }

    pub fn is_not(&self) -> bool {
        token(&self.0, SyntaxKind::IsKw).is_some() && token(&self.0, SyntaxKind::NotKw).is_some()
    }
}
