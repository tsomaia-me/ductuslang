#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct FlagRec {
    pub flag: char,
    pub lo: u32,
    pub hi: u32,
}
