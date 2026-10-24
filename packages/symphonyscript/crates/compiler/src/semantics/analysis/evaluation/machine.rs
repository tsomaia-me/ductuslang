use crate::semantics::analysis::call_key::CallKey;
use crate::semantics::analysis::cval::CVal;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::ir::FnId;

pub(crate) struct Frame {
    pub key: CallKey,
    pub function: FnId,
    pub args: Vec<CVal>,
    pub root: NodeId,
    pub ty: EvalTy,
    pub memo: FastMap<(NodeId, EvalTy), CVal>,
}

#[derive(Default)]
pub(crate) struct Machine {
    pub frames: Vec<Frame>,
    pub work: Vec<(usize, NodeId, EvalTy)>,
}
