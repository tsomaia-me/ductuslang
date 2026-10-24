use super::ast_node::AstNode;
use super::fn_body::FnBody;
use super::name::Name;
use super::param::Param;
use super::param_list::ParamList;
use super::support::{child, children, name_at};
use crate::syntax::{SyntaxKind, SyntaxNode};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct FnDecl(SyntaxNode);

impl AstNode for FnDecl {
    fn cast(node: SyntaxNode) -> Option<Self> {
        (node.kind() == SyntaxKind::FnDecl).then_some(Self(node))
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl FnDecl {
    pub fn name(&self) -> Option<Name> {
        name_at(&self.0, 0)
    }

    pub fn params(&self) -> impl Iterator<Item = Param> + use<> {
        child::<ParamList>(&self.0)
            .into_iter()
            .flat_map(|list| children::<Param>(list.syntax()))
    }

    pub fn body(&self) -> Option<FnBody> {
        child(&self.0)
    }
}
