#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct LetRec {
    pub binder: u32,
    pub value: u32,
    pub owner: u32,
    pub lo: u32,
    pub hi: u32,
    pub is_match: bool,
    pub faulty_line: bool,
}
