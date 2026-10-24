use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn is_chosen_float(ty: EvalTy) -> bool {
    matches!(ty, EvalTy::DefaultF32 | EvalTy::DefaultF64)
}
