use super::checks::Checks;
use super::frame_values::FrameValues;
use super::loop_run::LoopRun;
use super::mode::Mode;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::feed::Feed;
use crate::semantics::analysis::ids::{CallId, NodeId, ScopeId};
use crate::semantics::analysis::sink::Sink;
use crate::semantics::analysis::slot_owner::SlotOwner;

pub(crate) struct Frame {
    pub run: Option<LoopRun>,
    pub args: Vec<CVal>,
    pub call: NodeId,
    pub root: NodeId,
    pub ty: EvalTy,
    pub expected: EvalTy,
    pub feeds: Vec<Feed>,
    pub owner: SlotOwner,
    pub values: FrameValues,
    pub loops: FrameValues,
    pub held: Option<(NodeId, EvalTy, CallId)>,
    pub depth: u64,
    pub faults: FastSet<u32>,
    pub templates: FastSet<ScopeId>,
    pub checks: Checks,
    pub mode: Mode,
    pub sink: Sink,
    pub id: Option<CallId>,
    pub opened: FastSet<CallId>,
    pub untaken_depth: u64,
    pub untaken_start: usize,
    pub height: u64,
    pub callees: Vec<CallId>,
}
