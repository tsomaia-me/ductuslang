use super::answer::Answer;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::ir::ExprId;

#[derive(Default)]
pub(crate) struct ScopedReads {
    pub scopes: FastMap<ScopeId, FastMap<ExprId, Answer>>,
}
