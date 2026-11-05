use serde::Serialize;
use symphonyscript::{Diagnostic, Related};
use symphonyscript_core::Span;

use crate::engine::Engine;
use crate::finding::Finding;
use crate::signal::Signal;

#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct Verdict {
    pub id: serde_json::Value,
    pub engine: Option<Engine>,
    pub parses: bool,
    pub diagnostics: Vec<DiagnosticRecord>,
    pub signals: Vec<Signal>,
    pub findings: Vec<Finding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panic: Option<String>,
    pub elapsed_ms: Option<f64>,
    pub allocations: Option<u64>,
    pub peak_bytes: Option<u64>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct DiagnosticRecord {
    pub code: String,
    pub file: String,
    pub span: [u32; 2],
    pub message: String,
    pub help: Option<String>,
    pub related: Vec<RelatedRecord>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct RelatedRecord {
    pub file: String,
    pub span: [u32; 2],
    pub label: String,
}

impl From<&Diagnostic> for DiagnosticRecord {
    fn from(diagnostic: &Diagnostic) -> DiagnosticRecord {
        DiagnosticRecord {
            code: diagnostic.code.as_str().to_string(),
            file: diagnostic.span.file.clone(),
            span: offsets(&diagnostic.span),
            message: diagnostic.summary.clone(),
            help: diagnostic.help.clone(),
            related: diagnostic.related.iter().map(RelatedRecord::from).collect(),
        }
    }
}

impl From<&Related> for RelatedRecord {
    fn from(related: &Related) -> RelatedRecord {
        RelatedRecord {
            file: related.span.file.clone(),
            span: offsets(&related.span),
            label: related.label.clone(),
        }
    }
}

fn offsets(span: &Span) -> [u32; 2] {
    [span.range.start, span.range.end]
}
