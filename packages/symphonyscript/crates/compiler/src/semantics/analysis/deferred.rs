use super::chain::Chain;
use super::ids::{BindingId, NodeId};
use crate::diagnostics::Code;
use crate::semantics::ir::Loc;

pub(crate) struct Deferred {
    pub code: Code,
    pub loc: Loc,
    pub before: Vec<Loc>,
    pub chain: Chain,
    pub message: String,
    pub unless_untyped: Vec<BindingId>,
    pub unless_values: Vec<NodeId>,
}
