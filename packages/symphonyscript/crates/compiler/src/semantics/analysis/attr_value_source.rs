use super::ids::ScopeId;
use crate::semantics::ir::ExprId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AttrValueSource {
    Expr(ExprId, ScopeId),
    Set(bool),
}
