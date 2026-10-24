use super::ast_node::AstNode;
use super::expr::Expr;
use super::match_block::MatchBlock;
use super::support::child;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Destination<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Destination<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Destination).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Destination<'p> {
    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn match_block(&self) -> Option<MatchBlock<'p>> {
        child(&self.0)
    }
}
