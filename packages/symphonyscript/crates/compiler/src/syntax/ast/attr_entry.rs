use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at, token};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AttrEntry(SyntaxNode);

impl AstNode for AttrEntry {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::AttrEntry).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl AttrEntry {
    pub fn is_negated(&self) -> bool {
        token(&self.0, SyntaxKind::Bang).is_some()
    }

    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }
}
