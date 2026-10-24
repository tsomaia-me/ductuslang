use super::chain::Chain;
use crate::diagnostics::Code;
use crate::semantics::ir::Loc;

pub(crate) struct Deferred {
    pub code: Code,
    pub loc: Loc,
    pub before: Vec<Loc>,
    pub chain: Chain,
    pub message: String,
}
