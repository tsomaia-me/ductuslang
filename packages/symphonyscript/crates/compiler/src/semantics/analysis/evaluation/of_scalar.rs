use symphonyscript_core::Scalar;

use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn of_scalar(scalar: Scalar) -> EvalTy {
    match scalar {
        Scalar::I32 => EvalTy::I32,
        Scalar::I64 => EvalTy::I64,
        Scalar::F32 => EvalTy::F32,
        Scalar::F64 => EvalTy::F64,
        Scalar::Bool => EvalTy::Bool,
        Scalar::Enum(id) => EvalTy::Enum(id),
        Scalar::Str => EvalTy::Str,
    }
}
