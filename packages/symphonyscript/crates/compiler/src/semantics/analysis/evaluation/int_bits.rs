use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn int_bits(ty: EvalTy) -> Option<u32> {
    match ty {
        EvalTy::I32 => Some(32),
        EvalTy::I64 => Some(64),
        EvalTy::Exact
        | EvalTy::Default
        | EvalTy::F32
        | EvalTy::F64
        | EvalTy::DefaultF32
        | EvalTy::DefaultF64
        | EvalTy::Bool
        | EvalTy::Enum(_)
        | EvalTy::Str
        | EvalTy::Node => None,
    }
}
