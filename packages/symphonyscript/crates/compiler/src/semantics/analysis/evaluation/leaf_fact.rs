use symphonyscript_core::FamilyId;

use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;

#[derive(Clone, Copy, Debug)]
pub(crate) enum LeafFact {
    Value(bool, EvalTy, Option<FamilyId>),
    Unplaced(NodeId, EvalTy),
}
