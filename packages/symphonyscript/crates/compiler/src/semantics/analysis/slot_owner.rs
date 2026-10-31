use super::ids::{NodeId, ScopeId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum SlotOwner {
    Template(ScopeId),
    Root(NodeId),
}
