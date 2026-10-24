use super::ast_node::AstNode;
use super::name::Name;
use super::support::name_at;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct VariantRef<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for VariantRef<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::VariantRef).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> VariantRef<'p> {
    pub fn enum_name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn variant(&self) -> Option<Name<'p>> {
        name_at(&self.0, 1)
    }
}
