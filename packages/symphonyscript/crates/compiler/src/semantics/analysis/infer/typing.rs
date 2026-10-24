use symphonyscript_core::Scalar;

use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::ir::Loc;

#[derive(Default)]
pub(crate) struct Typing {
    pub expectations: FastMap<BindingId, Vec<(Scalar, Loc)>>,
    pub reads: FastMap<BindingId, Vec<Loc>>,
    pub own: FastMap<BindingId, Scalar>,
    pub synth: FastMap<NodeId, Option<Scalar>>,
    pub expected_seen: FastSet<(NodeId, Option<Scalar>)>,
    pub checked: FastSet<(NodeId, Option<Scalar>)>,
    pub natural: FastMap<BindingId, Option<Scalar>>,
    pub shapes: FastMap<(NodeId, Option<Scalar>), Option<Vec<u32>>>,
}
