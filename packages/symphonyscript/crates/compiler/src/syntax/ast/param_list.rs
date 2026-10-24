use super::ast_node::AstNode;
use super::param::Param;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ParamList(SyntaxNode);

impl AstNode for ParamList {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::ParamList).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl ParamList {
    pub fn params(&self) -> impl Iterator<Item = Param> + use<> {
        children(&self.0)
    }
}
