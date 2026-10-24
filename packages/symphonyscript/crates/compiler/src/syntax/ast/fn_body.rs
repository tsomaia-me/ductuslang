use super::ast_node::AstNode;
use super::expr::Expr;
use super::local_let::LocalLet;
use super::match_block::MatchBlock;
use super::support::{child, children};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FnBody<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for FnBody<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::FnBody).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> FnBody<'p> {
    pub fn lets(&self) -> impl Iterator<Item = LocalLet<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn match_block(&self) -> Option<MatchBlock<'p>> {
        child(&self.0)
    }
}
