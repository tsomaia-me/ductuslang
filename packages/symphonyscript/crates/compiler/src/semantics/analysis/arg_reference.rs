use std::rc::Rc;

use super::item_ref::ItemRef;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ArgReference {
    Unknown,
    Instance(ItemRef),
    Among(Rc<[ItemRef]>),
}
