use super::ast_node::AstNode;
use super::else_arm::ElseArm;
use super::expr::Expr;
use super::match_arm::MatchArm;
use super::support::{child, children};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MatchBlock(SyntaxNode);

impl AstNode for MatchBlock {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::MatchBlock).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl MatchBlock {
    pub fn scrutinee(&self) -> Option<Expr> {
        child(&self.0)
    }

    pub fn arms(&self) -> impl Iterator<Item = MatchArm> + use<> {
        children(&self.0)
    }

    pub fn else_arm(&self) -> Option<ElseArm> {
        child(&self.0)
    }
}
