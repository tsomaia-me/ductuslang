use alloc::boxed::Box;

use crate::ids::EnumId;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Ty {
    I32,
    I64,
    F32,
    F64,
    Bool,
    Enum(EnumId),
    Str,
    Array(Box<Ty>, u32),
}
