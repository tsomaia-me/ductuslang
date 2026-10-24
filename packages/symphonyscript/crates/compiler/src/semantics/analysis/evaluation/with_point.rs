pub(super) fn with_point(text: String) -> String {
    if text.contains('.') {
        text
    } else {
        format!("{text}.0")
    }
}
