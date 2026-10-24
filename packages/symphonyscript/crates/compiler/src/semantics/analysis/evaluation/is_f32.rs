use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn is_f32(ty: EvalTy) -> bool {
    matches!(ty, EvalTy::F32 | EvalTy::DefaultF32)
}
