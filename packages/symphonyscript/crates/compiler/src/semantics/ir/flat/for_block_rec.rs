use super::iteration::Iteration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ForBlockRec {
    pub head: Iteration,
    pub body: u32,
    pub lo: u32,
    pub hi: u32,
}
