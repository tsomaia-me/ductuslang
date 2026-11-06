#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PlacementRec {
    pub kind: u32,
    pub flags: (u32, u32),
    pub as_name: u32,
    pub has_as: bool,
    pub default: u32,
    pub gate: u32,
    pub attrs: (u32, u32),
    pub attrs_cut: bool,
    pub body: u32,
    pub skipped_block: Option<(u32, u32)>,
    pub inline: bool,
    pub lo: u32,
    pub hi: u32,
    pub faulty_line: bool,
}
