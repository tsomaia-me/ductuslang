use std::rc::Rc;

use symphonyscript_core::EnumId;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Value<A, R> {
    Int(i128),
    F32(f32),
    F64(f64),
    Bool(bool),
    Enum(EnumId, u16),
    Str(Rc<str>),
    Node(R),
    Array(A),
    Error,
}

impl<A, R> Value<A, R> {
    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error)
    }
}
