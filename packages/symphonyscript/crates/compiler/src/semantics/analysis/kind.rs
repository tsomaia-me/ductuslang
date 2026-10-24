use symphonyscript_core::EnumId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Kind {
    Int,
    Float,
    Bool,
    Enum(EnumId),
    Str,
    Node,
    Bare,
    Any,
    Error,
}
