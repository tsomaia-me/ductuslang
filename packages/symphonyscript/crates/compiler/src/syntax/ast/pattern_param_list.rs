use super::ast_node::AstNode;
use super::pattern_param::PatternParam;
use super::support::children;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PatternParamList(SyntaxNode);

impl AstNode for PatternParamList {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::PatternParamList).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PatternParamList {
    pub fn params(&self) -> impl Iterator<Item = PatternParam> + use<> {
        children(&self.0)
    }
}
