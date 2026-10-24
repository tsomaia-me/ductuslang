use crate::semantics::ir::{PlacementId, WireId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Site {
    Placement(PlacementId),
    Wire(WireId),
}
