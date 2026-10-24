use super::ast_node::AstNode;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Flags(SyntaxNode);

impl AstNode for Flags {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Flags).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Flags {
    pub fn chars(&self) -> Vec<char> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .flat_map(|token| match token.kind() {
                SyntaxKind::Star2 => vec!['*', '*'],
                _ => token.text().chars().collect(),
            })
            .collect()
    }
}
