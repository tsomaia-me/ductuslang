use crate::diagnostics::Diagnostic;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Failed {
    pub diagnostics: Vec<Diagnostic>,
}
