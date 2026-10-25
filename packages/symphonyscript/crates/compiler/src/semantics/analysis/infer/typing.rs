use symphonyscript_core::Scalar;

use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::ir::Loc;

#[derive(Default)]
pub(crate) struct Typing {
    pub expectations: FastMap<BindingId, Vec<(Scalar, Loc)>>,
    pub reads: FastMap<BindingId, Vec<Loc>>,
    pub own: FastMap<BindingId, Scalar>,
    pub untyped: FastSet<BindingId>,
    pub read_by_faults: FastSet<BindingId>,
    pub untyped_values: FastSet<NodeId>,
    pub awaiting_enum: Vec<BindingId>,
    pub enum_expected: Vec<BindingId>,
    pub equal_to: FastMap<BindingId, Vec<NodeId>>,
    pub synth: FastMap<NodeId, Option<Scalar>>,
    pub expected_seen: FastSet<(NodeId, Option<Scalar>)>,
    pub checked: FastSet<(NodeId, Option<Scalar>)>,
    pub evaluated: FastSet<(NodeId, Option<Scalar>)>,
    pub passed_over: FastSet<(NodeId, Option<Scalar>)>,
    pub passed_over_groups: FastSet<(NodeId, Option<Scalar>)>,
    pub natural: FastMap<BindingId, Option<Scalar>>,
    pub shapes: FastMap<(NodeId, Option<Scalar>), Option<Vec<u32>>>,
    #[cfg(test)]
    pub gate_positions: usize,
    #[cfg(test)]
    pub checks: usize,
}
