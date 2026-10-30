use super::ids::{NodeId, ScopeId};
use super::loop_types::LoopTypes;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum LoopTemplate {
    Trying {
        scope: ScopeId,
        types: LoopTypes,
        round: u8,
    },
    Final(Option<NodeId>),
}
