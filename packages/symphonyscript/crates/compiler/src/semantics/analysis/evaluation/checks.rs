use crate::semantics::analysis::ids::NodeId;

pub(crate) enum Checks {
    Unchecked,
    Checking { start: NodeId, pending: Vec<NodeId> },
    Done,
}
