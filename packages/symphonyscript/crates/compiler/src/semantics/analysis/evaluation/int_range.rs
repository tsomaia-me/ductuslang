use std::ops::RangeInclusive;

use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn int_range(ty: EvalTy) -> Option<RangeInclusive<i128>> {
    match ty {
        EvalTy::U8 => Some(0..=i128::from(u8::MAX)),
        EvalTy::I32 => Some(i128::from(i32::MIN)..=i128::from(i32::MAX)),
        EvalTy::I64 => Some(i128::from(i64::MIN)..=i128::from(i64::MAX)),
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
