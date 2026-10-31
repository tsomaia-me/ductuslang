use super::deferred::Deferred;
use super::reachable::Reachable;

pub(crate) enum FaultState {
    Recorded {
        faults: Vec<Deferred>,
        edges: Vec<Reachable>,
    },
    Drawn,
}
