use super::eval_ty::EvalTy;
use super::value::Value;

pub(crate) fn convert<A, R>(value: Value<A, R>, target: EvalTy) -> Value<A, R> {
    let target = match target {
        EvalTy::DefaultF32 => EvalTy::F32,
        EvalTy::DefaultF64 => EvalTy::F64,
        other => other,
    };
    match (value, target) {
        (Value::Int(value), EvalTy::U8) => Value::Int(value.clamp(0, i128::from(u8::MAX))),
        (Value::F32(value), EvalTy::U8) => Value::Int(i128::from(value as u8)),
        (Value::F64(value), EvalTy::U8) => Value::Int(i128::from(value as u8)),
        (Value::Int(value), EvalTy::I32) => {
            Value::Int(value.clamp(i128::from(i32::MIN), i128::from(i32::MAX)))
        }
        (Value::Int(value), EvalTy::I64) => {
            Value::Int(value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)))
        }
        (Value::Int(value), EvalTy::F32) => Value::F32(value as f32),
        (Value::Int(value), EvalTy::F64) => Value::F64(value as f64),
        (Value::F32(value), EvalTy::I32) => Value::Int(i128::from(value as i32)),
        (Value::F32(value), EvalTy::I64) => Value::Int(i128::from(value as i64)),
        (Value::F64(value), EvalTy::I32) => Value::Int(i128::from(value as i32)),
        (Value::F64(value), EvalTy::I64) => Value::Int(i128::from(value as i64)),
        (Value::F32(value), EvalTy::F64) => Value::F64(f64::from(value)),
        (Value::F64(value), EvalTy::F32) => Value::F32(value as f32),
        (value, _) => value,
    }
}
