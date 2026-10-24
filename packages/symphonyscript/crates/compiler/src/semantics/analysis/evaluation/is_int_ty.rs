use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn is_int_ty(ty: EvalTy) -> bool {
    matches!(ty, EvalTy::Exact | EvalTy::I32 | EvalTy::I64)
}
