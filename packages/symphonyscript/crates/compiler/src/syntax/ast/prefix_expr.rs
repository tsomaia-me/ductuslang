use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PrefixExpr(SyntaxNode);

impl AstNode for PrefixExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::PrefixExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PrefixExpr {
    pub fn op(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| !token.kind().is_trivia())
    }

    pub fn operand(&self) -> Option<Expr> {
        child(&self.0)
    }
}
