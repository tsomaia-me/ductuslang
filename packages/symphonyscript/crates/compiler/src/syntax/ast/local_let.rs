use super::ast_node::AstNode;
use super::expr::Expr;
use super::match_block::MatchBlock;
use super::name::Name;
use super::support::{child, name_at};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LocalLet(SyntaxNode);

impl AstNode for LocalLet {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::LocalLet).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl LocalLet {
    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn match_block(&self) -> Option<MatchBlock> {
        child(&self.0)
    }
}
