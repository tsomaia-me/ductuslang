use std::rc::Rc;

use super::leaf_fact::LeafFact;

#[derive(Clone, Debug)]
pub(crate) enum ArgFact {
    Default,
    Error,
    Leaf(LeafFact),
    Elements(Rc<[LeafFact]>, Rc<[u32]>),
}
