#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Leaf {
    Any,
    Error,
    Bool,
    Int,
    Float,
}
