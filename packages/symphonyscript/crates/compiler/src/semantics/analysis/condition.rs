use super::ids::ScopeId;
use crate::semantics::ir::ExprId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Condition {
    pub expr: ExprId,
    pub scope: ScopeId,
    pub holds: bool,
}
