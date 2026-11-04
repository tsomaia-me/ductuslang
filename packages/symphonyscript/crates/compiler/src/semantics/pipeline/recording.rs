use super::memo_key::MemoKey;
use crate::diagnostics::Diagnostic;

pub(crate) struct Recording {
    pub key: MemoKey,
    pub frames: usize,
    pub depth: usize,
    pub steps: u64,
    pub faults: Vec<Diagnostic>,
    pub storable: bool,
}
