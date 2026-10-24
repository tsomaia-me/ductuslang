use super::ast_node::AstNode;
use super::expr::Expr;
use super::match_block::MatchBlock;
use super::name::Name;
use super::support::{child, name_at};
use super::type_ref::TypeRef;
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LetDecl<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for LetDecl<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::LetDecl).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> LetDecl<'p> {
    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn ty(&self) -> Option<TypeRef<'p>> {
        child(&self.0)
    }

    pub fn value(&self) -> Option<Expr<'p>> {
        child(&self.0)
    }

    pub fn match_block(&self) -> Option<MatchBlock<'p>> {
        child(&self.0)
    }
}
