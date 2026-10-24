use symphonyscript_core::AttrId;

use super::item_ref::ItemRef;
use crate::semantics::ir::InputId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BindingKind {
    Input { input: InputId, element: u32 },
    Let,
    Param,
    Attr { item: ItemRef, attr: AttrId },
}
