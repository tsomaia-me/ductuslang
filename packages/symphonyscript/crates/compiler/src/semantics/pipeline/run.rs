use crate::diagnostics::Diagnostic;

pub(crate) struct Run {
    pub diagnostics: Vec<Diagnostic>,
    pub placeholders: Vec<u32>,
}
