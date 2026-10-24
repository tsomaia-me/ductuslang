use super::ast_node::AstNode;
use super::interpolation::Interpolation;
use super::string_part::StringPart;
use super::support::token;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct StringLit(SyntaxNode);

impl AstNode for StringLit {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::StringLit).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl StringLit {
    pub fn parts(&self) -> impl Iterator<Item = StringPart> + use<> {
        self.0
            .children_with_tokens()
            .filter_map(|element| match element {
                rowan::NodeOrToken::Token(token) if token.kind() == SyntaxKind::StringText => {
                    Some(StringPart::Text(token))
                }
                rowan::NodeOrToken::Node(node) => {
                    Interpolation::cast(node).map(StringPart::Interpolation)
                }
                rowan::NodeOrToken::Token(_) => None,
            })
    }

    pub fn is_terminated(&self) -> bool {
        token(&self.0, SyntaxKind::StringEnd).is_some()
    }
}
