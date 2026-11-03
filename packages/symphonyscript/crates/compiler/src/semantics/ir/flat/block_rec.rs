#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct BlockRec {
    pub items: (u32, u32),
    pub members: (u32, u32),
    pub regions: (u32, u32),
    pub lets: (u32, u32),
}
