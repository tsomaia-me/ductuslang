use super::float_text::float_text;
use super::float32_text::float32_text;
use crate::semantics::analysis::c_val::CVal;

pub(super) fn text(
    value: &CVal,
    variant: impl Fn(symphonyscript_core::EnumId, u16) -> String,
) -> Option<String> {
    Some(match value {
        CVal::Int(value) => value.to_string(),
        CVal::F32(value) => float32_text(*value),
        CVal::F64(value) => float_text(*value),
        CVal::Bool(value) => value.to_string(),
        CVal::Enum(id, index) => variant(*id, *index),
        CVal::Str(text) => text.to_string(),
        CVal::Node(_) | CVal::Error => return None,
    })
}
