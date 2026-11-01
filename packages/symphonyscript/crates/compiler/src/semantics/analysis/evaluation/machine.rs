use super::frame::Frame;
use super::frame_spare::FrameSpare;
use super::resume::Resume;
use crate::semantics::analysis::c_val::CVal;
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
    pub passed: Option<usize>,
    pub deep_warned: bool,
    pub walked: FastSet<NodeId>,
    pub spares: Vec<FrameSpare>,
    pub stack: Vec<CVal>,
    pub resumes: Vec<Resume>,
}
