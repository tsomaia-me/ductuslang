use super::i64_position::I64Position;

pub(crate) fn outside_i64(position: I64Position, value: i128) -> String {
    let (article, what) = match position {
        I64Position::Index => ("an", "index"),
        I64Position::Offset => ("an", "offset after `^`"),
        I64Position::SliceBound => ("a", "slice bound"),
    };
    format!(
        "the {what} {value} is outside `i64`, and {article} {what} known at compile time fits `i64`"
    )
}
