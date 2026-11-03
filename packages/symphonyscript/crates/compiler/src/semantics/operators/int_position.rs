use super::value::Value;

pub(crate) fn int_position<A, R>(value: &Value<A, R>) -> Option<i128> {
    match value {
        Value::Int(position) => Some(*position),
        Value::F32(_)
        | Value::F64(_)
        | Value::Bool(_)
        | Value::Enum(..)
        | Value::Str(_)
        | Value::Node(_)
        | Value::Array(_)
        | Value::Error => None,
    }
}
