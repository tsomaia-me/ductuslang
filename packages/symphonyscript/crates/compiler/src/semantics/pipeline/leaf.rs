#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Leaf {
    Placeholder,
    Error,
    Empty,
    Bool,
    Int,
    Float,
}
