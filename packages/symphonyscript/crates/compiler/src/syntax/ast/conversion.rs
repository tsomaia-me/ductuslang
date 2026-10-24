use super::ast_node::AstNode;
use super::expr::Expr;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Conversion(SyntaxNode);

impl AstNode for Conversion {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Conversion).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Conversion {
    pub fn target(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::I32Kw | SyntaxKind::I64Kw | SyntaxKind::F32Kw | SyntaxKind::F64Kw
                )
            })
    }

    pub fn operand(&self) -> Option<Expr> {
        child(&self.0)
    }
}
