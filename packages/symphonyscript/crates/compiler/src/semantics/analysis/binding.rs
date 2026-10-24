use symphonyscript_core::{AttrId, Scalar};

use super::ids::NodeId;
use super::item_ref::ItemRef;
use crate::semantics::ir::{InputId, Loc};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BindingKind {
    Input { input: InputId, element: u32 },
    Let,
    Param,
    Attr { item: ItemRef, attr: AttrId },
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Binding {
    pub kind: BindingKind,
    pub init: NodeId,
    pub annotation: Option<Scalar>,
    pub name: String,
    pub loc: Loc,
}
