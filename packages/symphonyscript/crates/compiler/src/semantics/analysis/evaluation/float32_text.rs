use super::with_point::with_point;

pub(super) fn float32_text(value: f32) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    with_point(format!("{value}"))
}
