use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn is_float_ty(ty: EvalTy) -> bool {
    matches!(
        ty,
        EvalTy::F32 | EvalTy::F64 | EvalTy::DefaultF32 | EvalTy::DefaultF64
    )
}
