use crate::ids::EnumId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Scalar {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Enum(EnumId),
    Str,
}
