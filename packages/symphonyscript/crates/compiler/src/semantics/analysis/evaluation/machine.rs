use super::frame::Frame;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::call_key::CallKey;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::ir::FnId;

#[derive(Default)]
pub(crate) struct Machine {
    pub frames: Vec<Frame>,
    pub work: Vec<(usize, NodeId, EvalTy)>,
    pub open: FastMap<FnId, u64>,
    pub open_keys: FastSet<CallKey>,
    pub too_deep: bool,
    pub deep_warned: bool,
    pub tainted_calls: FastMap<CallKey, CVal>,
}
