use crate::diagnostics::Code;

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct ShapeNeed {
    pub lengths: Vec<u32>,
    pub code: Code,
    pub message: &'static str,
}
