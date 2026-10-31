use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::{CallId, NodeId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum MachineKey {
    Call(CallId),
    Loop(NodeId, EvalTy),
}
