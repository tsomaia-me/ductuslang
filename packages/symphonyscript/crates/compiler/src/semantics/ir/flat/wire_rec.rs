#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct WireRec {
    pub kind: u32,
    pub flags: (u32, u32),
    pub as_name: u32,
    pub default: u32,
    pub gate: u32,
    pub attrs: (u32, u32),
    pub attrs_cut: bool,
    pub destination: u32,
    pub lo: u32,
    pub hi: u32,
}
