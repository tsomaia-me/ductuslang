use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;

#[derive(Clone, Copy, Debug)]
pub(crate) enum OperandPost {
    Keep,
    FitThenFloat(NodeId, EvalTy, EvalTy),
    Float(NodeId, bool, EvalTy),
}
