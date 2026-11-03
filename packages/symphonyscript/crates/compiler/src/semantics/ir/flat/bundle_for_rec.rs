use super::iteration::Iteration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct BundleForRec {
    pub head: Iteration,
    pub members: (u32, u32),
    pub lo: u32,
    pub hi: u32,
}
