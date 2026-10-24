use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Param(SyntaxNode);

impl AstNode for Param {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Param).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Param {
    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn default_value(&self) -> Option<Expr> {
        child(&self.0)
    }
}
