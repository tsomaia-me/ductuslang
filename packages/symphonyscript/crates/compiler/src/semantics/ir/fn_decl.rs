use super::fn_bound::FnBound;
use super::ident::Ident;
use super::ids::ExprId;
use super::let_decl::LetDecl;
use super::loc::Loc;
use super::param::Param;
use super::unknown_names::UnknownNames;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct FnDecl {
    pub name: Option<Ident>,
    pub params: Vec<Param>,
    pub skipped_params: UnknownNames,
    pub faulty_head: bool,
    pub cut: bool,
    pub bound: Option<FnBound>,
    pub lets: Vec<LetDecl>,
    pub value: ExprId,
    pub loc: Loc,
}
