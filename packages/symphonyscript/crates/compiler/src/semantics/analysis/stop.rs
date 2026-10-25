use crate::diagnostics::Code;
use crate::semantics::ir::Loc;

pub(crate) struct Stop {
    pub reported: usize,
    pub deferred: usize,
    pub inferred: bool,
    pub code: Code,
    pub loc: Loc,
    pub secondary: Vec<Loc>,
    pub message: String,
}
