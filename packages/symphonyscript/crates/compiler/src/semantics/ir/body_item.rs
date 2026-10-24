use super::bundle_for::BundleFor;
use super::ident::Ident;
use super::ids::{BlockId, PlacementId, WireId};
use super::iterable::Iterable;
use super::loc::Loc;
use super::when_arm::WhenArm;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum BodyItem {
    Placement(PlacementId),
    Wire(WireId),
    Bundle {
        members: Vec<PlacementId>,
        each: Option<BundleFor>,
        loc: Loc,
    },
    For {
        var: Option<Ident>,
        iterable: Iterable,
        body: BlockId,
        loc: Loc,
    },
    When {
        arms: Vec<WhenArm>,
        otherwise: Option<BlockId>,
        loc: Loc,
    },
}
