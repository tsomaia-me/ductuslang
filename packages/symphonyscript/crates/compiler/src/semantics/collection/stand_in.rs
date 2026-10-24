use crate::semantics::ir::PlacementId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StandIn {
    Node(PlacementId),
    Use(PlacementId),
}

impl StandIn {
    pub fn placement(self) -> PlacementId {
        match self {
            StandIn::Node(id) | StandIn::Use(id) => id,
        }
    }
}
