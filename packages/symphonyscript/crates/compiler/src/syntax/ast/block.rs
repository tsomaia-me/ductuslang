use super::ast_node::AstNode;
use super::body_item::BodyItem;
use super::local_let::LocalLet;
use super::members::Members;
use super::support::{child, children};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Block<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for Block<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::Block).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> Block<'p> {
    pub fn lets(&self) -> impl Iterator<Item = LocalLet<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn items(&self) -> impl Iterator<Item = BodyItem<'p>> + use<'p> {
        children(&self.0)
    }

    pub fn members(&self) -> Option<Members<'p>> {
        child(&self.0)
    }
}
