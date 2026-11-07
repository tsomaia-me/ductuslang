#[derive(Clone, Copy, Debug)]
pub(crate) struct AttrItem {
    pub file: u32,
    pub default: u32,
    pub attrs: (u32, u32),
    pub flags: (u32, u32),
    pub lo: u32,
    pub hi: u32,
}
