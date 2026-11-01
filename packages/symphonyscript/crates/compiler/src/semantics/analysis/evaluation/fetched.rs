use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Fetched {
    Raw(NodeId, EvalTy, Option<EvalTy>),
    Typed(NodeId, EvalTy, bool),
}
