use symphonyscript_core::EnumId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum EvalTy {
    Exact,
    Default,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Enum(EnumId),
    Str,
    Node,
}
