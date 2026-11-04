use super::convert::convert;
use super::eval_ty::EvalTy;
use super::is_float_ty::is_float_ty;
use super::value::Value;

pub(crate) fn convert_to<A, R>(value: Value<A, R>, ty: EvalTy) -> Value<A, R> {
    match (&value, ty) {
        (Value::Int(_), ty) if is_float_ty(ty) => convert(value, ty),
        (Value::F32(_), EvalTy::F64 | EvalTy::DefaultF64) => convert(value, ty),
        (Value::F32(_) | Value::F64(_), EvalTy::U8 | EvalTy::I32 | EvalTy::I64 | EvalTy::Exact) => {
            Value::Error
        }
        _ => value,
    }
}
