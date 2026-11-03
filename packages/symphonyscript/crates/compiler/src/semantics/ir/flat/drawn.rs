use crate::diagnostics::Code;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Drawn {
    pub code: Code,
    pub lo: u32,
    pub hi: u32,
    pub summary: (u32, u32),
}
