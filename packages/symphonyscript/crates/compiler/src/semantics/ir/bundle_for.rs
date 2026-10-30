use super::ids::PlacementId;
use super::iterable::Iterable;
use super::loc::Loc;
use super::loop_var::LoopVar;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct BundleFor {
    pub names: Vec<LoopVar>,
    pub iterable: Iterable,
    pub members: Vec<PlacementId>,
    pub loc: Loc,
}
