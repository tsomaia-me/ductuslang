use super::ident::Ident;
use super::ids::PlacementId;
use super::iterable::Iterable;
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct BundleFor {
    pub var: Option<Ident>,
    pub iterable: Iterable,
    pub members: Vec<PlacementId>,
    pub loc: Loc,
}
