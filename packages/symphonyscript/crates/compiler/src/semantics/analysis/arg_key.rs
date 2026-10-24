use std::rc::Rc;

use symphonyscript_core::EnumId;

use super::eval_ty::EvalTy;
use super::item_ref::ItemRef;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ArgKey {
    Int(i128, EvalTy),
    F32(u32),
    F64(u64),
    Bool(bool),
    Enum(EnumId, u16),
    Str(Rc<str>),
    Node(ItemRef),
    Array(Vec<ArgKey>),
    Default,
    Error,
}
