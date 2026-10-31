use super::frame::Frame;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::ids::{CallId, NodeId};

#[derive(Default)]
pub(crate) struct Machine {
    pub frames: Vec<Frame>,
    pub work: Vec<(usize, NodeId, EvalTy)>,
    pub open_keys: FastSet<CallId>,
    pub rerun: FastSet<CallId>,
    pub too_deep: bool,
    pub deep_warned: bool,
}
