use super::ids::{NodeId, ScopeId};
use super::loop_types::LoopTypes;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum LoopTemplate {
    Trial {
        scope: ScopeId,
        types: LoopTypes,
        round: u8,
    },
    Settled {
        scope: ScopeId,
        types: LoopTypes,
    },
    Final(Option<NodeId>),
}
