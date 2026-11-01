use std::rc::Rc;

use crate::semantics::analysis::ids::NodeId;

#[derive(Clone, Debug)]
pub(crate) enum ConcatPart {
    Text(Rc<str>),
    Value(NodeId, bool),
}
