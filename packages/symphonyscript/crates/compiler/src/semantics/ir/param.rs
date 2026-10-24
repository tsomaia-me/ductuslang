use super::ident::Ident;
use super::ids::ExprId;
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Param {
    pub name: Option<Ident>,
    pub default: Option<ExprId>,
    pub is_default: bool,
    pub loc: Loc,
}
