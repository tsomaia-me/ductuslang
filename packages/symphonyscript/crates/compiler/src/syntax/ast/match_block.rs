use super::ast_node::AstNode;
use super::else_arm::ElseArm;
use super::expr::Expr;
use super::match_arm::MatchArm;
use super::support::{child, children};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MatchBlock<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for MatchBlock<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::MatchBlock).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> MatchBlock<'p> {
    pub fn scrutinee(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn arms(&self) -> impl Iterator<Item = MatchArm<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn else_arm(&self) -> Option<ElseArm<'p>> {
        child(&self.0)
    }
}
