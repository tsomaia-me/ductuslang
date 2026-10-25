use super::ids::ExprId;
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct FnBound {
    pub bound: Option<u64>,
    pub fallback: ExprId,
    pub loc: Loc,
}
