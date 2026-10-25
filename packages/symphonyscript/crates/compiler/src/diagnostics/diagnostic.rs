use symphonyscript_core::Span;

use super::code::Code;
use super::related::Related;
use super::severity::Severity;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Diagnostic {
    pub code: Code,
    pub summary: String,
    pub help: Option<String>,
    pub span: Span,
    pub related: Vec<Related>,
}

impl Diagnostic {
    pub fn new(code: Code, span: Span, summary: impl Into<String>) -> Self {
        Self {
            code,
            summary: summary.into(),
            help: code.help().map(str::to_string),
            span,
            related: Vec::new(),
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn with_related(mut self, related: Vec<Related>) -> Self {
        self.related = related;
        self
    }

    pub fn severity(&self) -> Severity {
        self.code.severity()
    }
}
