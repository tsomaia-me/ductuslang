use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use super::support::{child, name_at, token};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PatternParam(SyntaxNode);

impl AstNode for PatternParam {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::PatternParam).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl PatternParam {
    pub fn is_default(&self) -> bool {
        token(&self.0, SyntaxKind::DefaultKw).is_some()
    }

    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn default_value(&self) -> Option<Expr> {
        child(&self.0)
    }
}
