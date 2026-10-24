use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::call_key::CallKey;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::feed::Feed;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::ir::FnId;

pub(crate) struct Frame {
    pub key: CallKey,
    pub function: FnId,
    pub args: Vec<CVal>,
    pub call: NodeId,
    pub root: NodeId,
    pub ty: EvalTy,
    pub expected: EvalTy,
    pub feeds: Vec<Feed>,
    pub memo: FastMap<(NodeId, EvalTy), CVal>,
}
