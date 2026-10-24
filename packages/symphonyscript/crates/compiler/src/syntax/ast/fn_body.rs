use super::ast_node::AstNode;
use super::expr::Expr;
use super::local_let::LocalLet;
use super::match_block::MatchBlock;
use super::support::{child, children};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FnBody(SyntaxNode);

impl AstNode for FnBody {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::FnBody).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl FnBody {
    pub fn lets(&self) -> impl Iterator<Item = LocalLet> + use<> {
        children(&self.0)
    }

    pub fn value(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn match_block(&self) -> Option<MatchBlock> {
        child(&self.0)
    }
}
