use super::ident::Ident;
use super::ids::{BlockId, ExprId, PlacementId, WireId};
use super::iterable::Iterable;
use super::loc::Loc;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct BundleFor {
    pub var: Option<Ident>,
    pub iterable: Iterable,
    pub members: Vec<PlacementId>,
    pub loc: Loc,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct WhenArm {
    pub guard: ExprId,
    pub body: BlockId,
}

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
