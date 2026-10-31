use super::ast_node::AstNode;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Flags<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Flags<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Flags).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Flags<'p> {
    pub fn chars(&self) -> Vec<char> {
        self.0
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .flat_map(|token| match token.kind() {
                SyntaxKind::Star2 => vec!['*', '*'],
                SyntaxKind::Question2 => vec!['?', '?'],
                _ => token.text().chars().collect(),
            })
            .collect()
    }
}
