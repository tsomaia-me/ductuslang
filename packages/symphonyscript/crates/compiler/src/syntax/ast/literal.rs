use super::ast_node::AstNode;
use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Literal(SyntaxNode);

impl AstNode for Literal {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Literal).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Literal {
    pub fn token(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| !token.kind().is_trivia())
    }
}
