use super::bound_val::BoundVal;
use super::ids::ScopeId;
use crate::semantics::ir::ExprId;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum ParamSource {
    Arg { expr: ExprId, scope: ScopeId },
    Default,
    Missing,
    Bound(BoundVal),
}
