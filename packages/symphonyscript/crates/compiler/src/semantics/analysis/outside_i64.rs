pub(crate) fn outside_i64(value: i128) -> String {
    format!(
        "the slice bound {value} is outside `i64`, and a slice bound known at compile time fits `i64`"
    )
}
