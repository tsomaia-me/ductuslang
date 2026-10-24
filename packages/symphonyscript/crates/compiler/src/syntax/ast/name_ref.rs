use super::ast_node::AstNode;
use super::name::Name;
use super::support::name_at;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct NameRef(SyntaxNode);

impl AstNode for NameRef {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::NameRef).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl NameRef {
    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }
}
