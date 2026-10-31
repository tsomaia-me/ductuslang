use super::ast_node::AstNode;
use super::expr::Expr;
use super::lambda::Lambda;
use super::match_block::MatchBlock;
use super::name::Name;
use super::support::{child, name_at};
use super::r#type::Type;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LocalLet<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for LocalLet<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::LocalLet).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> LocalLet<'p> {
    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn ty(&self) -> Option<Type<'p>> {
        child(&self.0)
    }

    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn lambda(&self) -> Option<Lambda<'p>> {
        child(&self.0)
    }

    pub fn match_block(&self) -> Option<MatchBlock<'p>> {
        child(&self.0)
    }
}
