use crate::semantics::analysis::ids::NodeId;

pub(crate) enum Checks {
    Unchecked,
    Running { start: NodeId, pending: Vec<NodeId> },
    Done,
    Quiet,
}

impl Checks {
    pub(crate) fn quiet(&self) -> bool {
        match self {
            Checks::Running { .. } | Checks::Quiet => true,
            Checks::Unchecked | Checks::Done => false,
        }
    }
}
