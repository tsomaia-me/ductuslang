use serde::Serialize;
use symphonyscript::Diagnostic;

use crate::finding::Finding;
use crate::signal::Signal;

#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct Verdict {
    pub id: serde_json::Value,
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
}

impl From<&Diagnostic> for DiagnosticRecord {
    fn from(diagnostic: &Diagnostic) -> DiagnosticRecord {
        DiagnosticRecord {
            code: diagnostic.code.as_str().to_string(),
            file: diagnostic.span.file.clone(),
            span: [diagnostic.span.range.start, diagnostic.span.range.end],
            message: diagnostic.summary.clone(),
        }
    }
}
