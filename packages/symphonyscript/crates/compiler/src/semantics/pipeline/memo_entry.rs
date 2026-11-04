use super::core_value::CoreValue;
use crate::diagnostics::Diagnostic;

pub(crate) struct MemoEntry {
    pub value: CoreValue,
    pub faults: Vec<Diagnostic>,
    pub steps: u64,
}
