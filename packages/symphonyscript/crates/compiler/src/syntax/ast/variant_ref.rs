use super::ast_node::AstNode;
use super::name::Name;
use super::support::name_at;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct VariantRef(SyntaxNode);

impl AstNode for VariantRef {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::VariantRef).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl VariantRef {
    pub fn enum_name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn variant(&self) -> Option<Name> {
        name_at(&self.0, 1)
    }
}
