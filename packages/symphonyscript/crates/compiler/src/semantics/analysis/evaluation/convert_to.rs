use super::convert::convert;
use super::is_float_ty::is_float_ty;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::eval_ty::EvalTy;

pub(super) fn convert_to(value: CVal, ty: EvalTy) -> CVal {
    match (&value, ty) {
        (CVal::Int(_), ty) if is_float_ty(ty) => convert(value, ty),
        (CVal::F32(_), EvalTy::F64 | EvalTy::DefaultF64) => convert(value, ty),
        (CVal::F32(_) | CVal::F64(_), EvalTy::U8 | EvalTy::I32 | EvalTy::I64 | EvalTy::Exact) => {
            CVal::Error
        }
        _ => value,
    }
}
