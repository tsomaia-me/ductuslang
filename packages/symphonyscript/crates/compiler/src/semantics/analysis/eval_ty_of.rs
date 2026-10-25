use symphonyscript_core::Scalar;

use super::eval_ty::EvalTy;

pub(crate) fn eval_ty_of(scalar: Option<Scalar>) -> EvalTy {
    match scalar {
        Some(Scalar::U8) => EvalTy::U8,
        Some(Scalar::I32) => EvalTy::I32,
        Some(Scalar::I64) => EvalTy::I64,
        Some(Scalar::F32) => EvalTy::F32,
        Some(Scalar::F64) => EvalTy::F64,
        Some(Scalar::Bool) => EvalTy::Bool,
        Some(Scalar::Enum(id)) => EvalTy::Enum(id),
        Some(Scalar::Str) => EvalTy::Str,
        None => EvalTy::Default,
    }
}
