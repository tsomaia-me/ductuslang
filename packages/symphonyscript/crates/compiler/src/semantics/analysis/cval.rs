use std::rc::Rc;

use symphonyscript_core::EnumId;

use super::item_ref::ItemRef;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum CVal {
    Int(i128),
    F32(f32),
    F64(f64),
    Bool(bool),
    Enum(EnumId, u16),
    Str(Rc<str>),
    Node(ItemRef),
    Error,
}

impl CVal {
    pub fn is_error(&self) -> bool {
        matches!(self, CVal::Error)
    }
}
