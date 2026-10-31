use crate::diagnostics::Code;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::ir::Loc;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct ShapeNeed {
    pub lengths: Vec<u32>,
    pub code: Code,
    pub message: String,
    pub at: Option<(Loc, ScopeId)>,
}
