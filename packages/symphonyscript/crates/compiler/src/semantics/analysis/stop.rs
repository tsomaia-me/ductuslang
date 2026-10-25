use crate::diagnostics::Code;
use crate::semantics::ir::Loc;

pub(crate) struct Stop {
    pub reported: usize,
    pub deferred: usize,
    pub inferred: bool,
    pub code: Code,
    pub loc: Loc,
    pub related: Vec<(Loc, String)>,
    pub message: String,
}
