use super::bundle_for_rec::BundleForRec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct BundleRec {
    pub members: (u32, u32),
    pub each: Option<BundleForRec>,
    pub faulty: bool,
    pub start_touched: bool,
    pub lo: u32,
    pub hi: u32,
}
