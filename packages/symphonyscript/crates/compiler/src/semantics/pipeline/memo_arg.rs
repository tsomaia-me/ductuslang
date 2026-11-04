use symphonyscript_core::EnumId;

use super::core_value::CoreValue;
use crate::semantics::operators::Value;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum MemoArg {
    Int(i128),
    F32(u32),
    F64(u64),
    Bool(bool),
    Enum(EnumId, u16),
    Node(u32),
    Array(u32),
}

impl MemoArg {
    pub fn of(value: &CoreValue) -> Option<MemoArg> {
        Some(match value {
            Value::Int(int) => MemoArg::Int(*int),
            Value::F32(float) => MemoArg::F32(float.to_bits()),
            Value::F64(float) => MemoArg::F64(float.to_bits()),
            Value::Bool(bool) => MemoArg::Bool(*bool),
            Value::Enum(id, variant) => MemoArg::Enum(*id, *variant),
            Value::Node(node) => MemoArg::Node(*node),
            Value::Array(array) => MemoArg::Array(*array),
            Value::Str(_) | Value::Error => return None,
        })
    }
}
