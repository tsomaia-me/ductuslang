use super::ast_node::AstNode;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ToExpr(SyntaxNode);

impl AstNode for ToExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::ToExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}
