use super::ast_node::AstNode;
use super::expr::Expr;
use super::match_block::MatchBlock;
use super::support::child;
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Destination(SyntaxNode);

impl AstNode for Destination {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Destination).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Destination {
    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn match_block(&self) -> Option<MatchBlock> {
        child(&self.0)
    }
}
