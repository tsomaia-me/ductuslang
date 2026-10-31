use super::eval_ty::EvalTy;
use super::ids::{CallId, NodeId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Reachable {
    Eval(NodeId, EvalTy),
    Call(CallId),
    Loop(NodeId, EvalTy),
}
