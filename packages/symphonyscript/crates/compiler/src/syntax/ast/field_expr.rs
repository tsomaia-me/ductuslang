use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FieldExpr(SyntaxNode);

impl AstNode for FieldExpr {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::FieldExpr).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl FieldExpr {
    pub fn receiver(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn field(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }
}
