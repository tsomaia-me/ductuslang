use std::cmp::Ordering;

use symphonyscript_core::BinaryOp;

use super::value::Value;

pub(crate) fn compare<A, R>(op: BinaryOp, a: &Value<A, R>, b: &Value<A, R>) -> Value<A, R> {
    let ordering = match (a, b) {
        (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
        (Value::F32(a), Value::F32(b)) => a.partial_cmp(b),
        (Value::F64(a), Value::F64(b)) => a.partial_cmp(b),
        (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
        (Value::Enum(x, a), Value::Enum(y, b)) if x == y => a.partial_cmp(b),
        (Value::Str(a), Value::Str(b)) => a.partial_cmp(b),
        _ => return Value::Error,
    };
    let result = match op {
        BinaryOp::Is => ordering == Some(Ordering::Equal),
        BinaryOp::IsNot => ordering != Some(Ordering::Equal),
        BinaryOp::Lt => ordering == Some(Ordering::Less),
        BinaryOp::Le => matches!(ordering, Some(Ordering::Less | Ordering::Equal)),
        BinaryOp::Gt => ordering == Some(Ordering::Greater),
        BinaryOp::Ge => matches!(ordering, Some(Ordering::Greater | Ordering::Equal)),
        BinaryOp::Add
        | BinaryOp::Sub
        | BinaryOp::Mul
        | BinaryOp::Div
        | BinaryOp::IntDiv
        | BinaryOp::Rem
        | BinaryOp::Pow
        | BinaryOp::And
        | BinaryOp::Or => return Value::Error,
    };
    Value::Bool(result)
}
