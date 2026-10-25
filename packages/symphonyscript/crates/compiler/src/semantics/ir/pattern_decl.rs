use super::ident::Ident;
use super::ids::BlockId;
use super::let_decl::LetDecl;
use super::loc::Loc;
use super::param::Param;
use super::unknown_names::UnknownNames;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct PatternDecl {
    pub name: Option<Ident>,
    pub params: Vec<Param>,
    pub skipped_params: UnknownNames,
    pub skipped_default: bool,
    pub lets: Vec<LetDecl>,
    pub body: BlockId,
    pub loc: Loc,
}
