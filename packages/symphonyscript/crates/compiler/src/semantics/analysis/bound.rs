use super::ids::{BindingId, NodeId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Bound {
    Cell(BindingId),
    Value(NodeId),
}
