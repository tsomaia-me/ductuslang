use crate::semantics::ir::CellWord;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct CellRec {
    pub binder: u32,
    pub word: CellWord,
    pub input: bool,
    pub init: u32,
    pub clock: (u32, u32),
    pub rule: u32,
    pub capacity: Option<u32>,
    pub lo: u32,
    pub hi: u32,
    pub drawn: (u32, u32),
    pub faulty_line: bool,
}
