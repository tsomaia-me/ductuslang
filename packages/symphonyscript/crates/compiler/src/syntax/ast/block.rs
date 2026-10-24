use super::ast_node::AstNode;
use super::body_item::BodyItem;
use super::local_let::LocalLet;
use super::members::Members;
use super::support::{child, children};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Block(SyntaxNode);

impl AstNode for Block {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::Block).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl Block {
    pub fn lets(&self) -> impl Iterator<Item = LocalLet> + use<> {
        children(&self.0)
    }

    pub fn items(&self) -> impl Iterator<Item = BodyItem> + use<> {
        children(&self.0)
    }

    pub fn members(&self) -> Option<Members> {
        child(&self.0)
    }
}
