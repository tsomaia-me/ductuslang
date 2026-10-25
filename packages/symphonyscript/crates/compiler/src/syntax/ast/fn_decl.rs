use super::ast_node::AstNode;
use super::bounded::Bounded;
use super::fn_body::FnBody;
use super::name::Name;
use super::param::Param;
use super::param_list::ParamList;
use super::support::{child, children, name_at};
use crate::syntax::{NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FnDecl<'p>(NodeView<'p>);

impl<'p> AstNode<'p> for FnDecl<'p> {
    fn cast(node: NodeView<'p>) -> Option<Self> {
        (node.kind() == SyntaxKind::FnDecl).then_some(Self(node))
    }

    fn syntax(&self) -> &NodeView<'p> {
        &self.0
    }
}

impl<'p> FnDecl<'p> {
    pub fn name(&self) -> Option<Name<'p>> {
        name_at(&self.0, 0)
    }

    pub fn params(&self) -> impl Iterator<Item = Param<'p>> + use<'p> {
        child::<ParamList>(&self.0)
            .into_iter()
            .flat_map(|list| children::<Param>(list.syntax()))
    }

    pub fn bounded(&self) -> Option<Bounded<'p>> {
        child(&self.0)
    }

    pub fn body(&self) -> Option<FnBody<'p>> {
        child(&self.0)
    }
}
