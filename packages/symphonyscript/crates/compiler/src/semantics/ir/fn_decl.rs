use super::ident::Ident;
use super::ids::ExprId;
use super::let_decl::LetDecl;
use super::loc::Loc;
use super::param::Param;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct FnDecl {
    pub name: Option<Ident>,
    pub params: Vec<Param>,
    pub lets: Vec<LetDecl>,
    pub value: ExprId,
    pub loc: Loc,
}
