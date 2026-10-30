use crate::semantics::analysis::call_key::CallKey;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum MachineKey {
    Call(CallKey),
    Loop(NodeId, EvalTy),
}
