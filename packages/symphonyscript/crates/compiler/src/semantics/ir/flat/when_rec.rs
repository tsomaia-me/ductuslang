#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct WhenRec {
    pub arms: (u32, u32),
    pub otherwise: u32,
    pub lo: u32,
    pub hi: u32,
}
