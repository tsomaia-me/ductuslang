use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct NamedArg(SyntaxNode);

impl AstNode for NamedArg {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::NamedArg).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl NamedArg {
    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }
}
