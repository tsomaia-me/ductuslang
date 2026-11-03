#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PatternRec {
    pub name: u32,
    pub params: (u32, u32),
    pub body: u32,
    pub regions: (u32, u32),
    pub skipped_default: bool,
    pub lo: u32,
    pub hi: u32,
    pub faulty_line: bool,
}
