use super::bundle_rec::BundleRec;
use super::for_block_rec::ForBlockRec;
use super::placement_rec::PlacementRec;
use super::when_rec::WhenRec;
use super::wire_rec::WireRec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ItemRec {
    Placement(PlacementRec),
    Wire(WireRec),
    Bundle(BundleRec),
    For(ForBlockRec),
    When(WhenRec),
    Line((u32, u32)),
    Paren(u32),
}
