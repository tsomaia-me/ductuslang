use super::ids::ScopeId;
use super::op::Op;
use crate::semantics::ir::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Node {
    pub op: Op,
    pub loc: Loc,
    pub scope: ScopeId,
    pub order: Option<(ScopeId, u32)>,
}
