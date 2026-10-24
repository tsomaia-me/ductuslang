use super::ast_node::AstNode;
use super::interpolation::Interpolation;
use super::string_part::StringPart;
use super::support::token;
use crate::syntax::{ElementView, NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct StringLit<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for StringLit<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::StringLit).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> StringLit<'p> {
    pub fn parts(&self) -> impl Iterator<Item = StringPart<'p>> + use<'p> {
        self.0
            .children_with_tokens()
            .filter_map(|element| match element {
                ElementView::Token(token) if token.kind() == SyntaxKind::StringText => {
                    Some(StringPart::Text(token))
                }
                ElementView::Node(node) => Interpolation::cast(node).map(StringPart::Interpolation),
                ElementView::Token(_) => None,
            })
    }

    pub fn is_terminated(&self) -> bool {
        token(&self.0, SyntaxKind::StringEnd).is_some()
    }
}
