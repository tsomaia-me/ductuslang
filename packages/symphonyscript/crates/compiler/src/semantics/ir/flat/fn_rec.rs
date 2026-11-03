#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct FnRec {
    pub name: u32,
    pub params: (u32, u32),
    pub lets: (u32, u32),
    pub value: u32,
    pub regions: (u32, u32),
    pub lo: u32,
    pub hi: u32,
    pub faulty_line: bool,
}
