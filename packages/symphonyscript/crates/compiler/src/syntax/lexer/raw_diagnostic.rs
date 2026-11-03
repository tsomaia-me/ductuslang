use std::ops::Range;

use symphonyscript_core::Span;

use crate::diagnostics::{Code, Diagnostic};

pub(super) struct RawDiagnostic {
    pub(super) code: Code,
    pub(super) range: Range<u32>,
    pub(super) summary: String,
    pub(super) help: Option<String>,
}

impl RawDiagnostic {
    pub(super) fn into_diagnostic(self, file: &str) -> Diagnostic {
        let span = Span {
            file: file.to_string(),
            range: self.range,
        };
        let diagnostic = Diagnostic::new(self.code, span, self.summary);
        match self.help {
            Some(help) => diagnostic.with_help(help),
            None => diagnostic,
        }
    }
}
