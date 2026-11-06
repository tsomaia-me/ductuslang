use symphonyscript_core::EnumId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Leaf {
    Placeholder,
    Error,
    Empty,
    Bool,
    Int,
    Float,
    Enum(Option<EnumId>),
}
