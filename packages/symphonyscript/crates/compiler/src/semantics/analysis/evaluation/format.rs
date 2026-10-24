use crate::semantics::analysis::cval::CVal;

pub(super) fn float_text(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    with_point(format!("{value}"))
}

pub(super) fn float32_text(value: f32) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    with_point(format!("{value}"))
}

fn with_point(text: String) -> String {
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}

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
