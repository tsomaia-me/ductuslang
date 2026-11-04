use super::c_array::CArray;
use super::item_ref::ItemRef;
use crate::semantics::operators::Value;

pub(crate) type CVal = Value<CArray, ItemRef>;
