use std::time::Duration;

use serde::{Deserialize, Serialize};
use symphonyscript::{Diagnostic, Severity};
use symphonyscript_core::Span;

use crate::guarded::{Guarded, guarded};

pub const PROBED_PATH: &str = "main.sym";

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Probed {
    Finished {
        elapsed_ms: f64,
        diagnostics: Vec<ProbedDiagnostic>,
    },
    Panicked {
        message: String,
    },
    TimedOut,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ProbedDiagnostic {
    pub code: String,
    pub error: bool,
    pub span: ProbedSpan,
    pub summary: String,
    pub help: Option<String>,
    pub related: Vec<ProbedRelated>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ProbedSpan {
    pub file: String,
    pub start: [usize; 2],
    pub end: [usize; 2],
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ProbedRelated {
    pub span: ProbedSpan,
    pub label: String,
}

pub fn probed<F>(budget: Duration, text: &str, check: F) -> Probed
where
    F: FnOnce() -> Vec<Diagnostic> + Send + 'static,
{
    match guarded(budget, check) {
        Guarded::Finished { value, elapsed, .. } => Probed::Finished {
            elapsed_ms: elapsed.as_secs_f64() * 1000.0,
            diagnostics: value
                .iter()
                .map(|diagnostic| ProbedDiagnostic {
                    code: diagnostic.code.as_str().to_string(),
                    error: diagnostic.severity() == Severity::Error,
                    span: probed_span(&diagnostic.span, text),
                    summary: diagnostic.summary.clone(),
                    help: diagnostic.help.clone(),
                    related: diagnostic
                        .related
                        .iter()
                        .map(|related| ProbedRelated {
                            span: probed_span(&related.span, text),
                            label: related.label.clone(),
                        })
                        .collect(),
                })
                .collect(),
        },
        Guarded::Panicked { message } => Probed::Panicked { message },
        Guarded::TimedOut => Probed::TimedOut,
    }
}

fn probed_span(span: &Span, text: &str) -> ProbedSpan {
    ProbedSpan {
        file: span.file.clone(),
        start: line_column(text, span.range.start),
        end: line_column(text, span.range.end),
    }
}

fn line_column(text: &str, offset: u32) -> [usize; 2] {
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    let mut line = 1;
    let mut column = 1;
    for (index, character) in text.char_indices() {
        if index >= offset {
            break;
        }
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    [line, column]
}
