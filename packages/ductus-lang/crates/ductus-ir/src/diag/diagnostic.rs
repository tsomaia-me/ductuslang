//! The `Diagnostic` struct: the single failure carrier every stage returns.

use super::diag_class::DiagClass;
use super::span::Span;
use crate::path::CellPath;

/// One diagnostic — the single carrier flowing through every stage's `Result`
/// (R10). Lexer, parser, loader and runtime all report this type; there is no
/// per-stage error family.
///
/// `span` is present when the failure can point at source text, `path` when it
/// can point at a cell. Either, both, or neither may apply.
///
/// There is deliberately no `PartialEq`. Whole-struct equality would fold
/// `message` into the comparison, so every assertion on a diagnostic would
/// quietly become an assertion on its wording — exactly what F3 rules out.
/// Tests compare `class`, `span` and `path` field by field, and never the
/// message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// What went wrong, as a stable class.
    pub class: DiagClass,
    /// Where in the source text, when known.
    pub span: Option<Span>,
    /// Which cell, when known.
    pub path: Option<CellPath>,
    /// Human-readable detail. Free to change; never asserted on by tests.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::Segment;

    // Byte offsets: `{` at 6, `\n` at 7, `a` at 10, `}` at 16, end at 18.
    const SOURCE: &str = "graph {\n  a = 1\n}\n";

    fn duplicate_cell_path() -> Diagnostic {
        Diagnostic {
            class: DiagClass::DuplicateCellPath,
            span: Some(Span::new(10, 1)),
            path: Some(CellPath::new(vec![
                Segment::Name {
                    text: "app".to_string(),
                    ordinal: None,
                },
                Segment::Name {
                    text: "gain".to_string(),
                    ordinal: None,
                },
            ])),
            message: "cell `app.gain` is declared twice".to_string(),
        }
    }

    #[test]
    fn diagnostic_carries_class_span_and_path() {
        let diagnostic = duplicate_cell_path();

        assert_eq!(diagnostic.class.id(), "duplicate_cell_path");
        assert_eq!(
            diagnostic.span.map(|span| span.line_col(SOURCE)),
            Some((2, 3))
        );
        assert_eq!(
            diagnostic.path.as_ref().map(CellPath::render).as_deref(),
            Some("app.gain")
        );
    }

    // Field by field, never whole-struct: `Diagnostic` has no `PartialEq`
    // precisely so a comparison can never smuggle the message into an
    // assertion.
    #[test]
    fn cloning_preserves_the_asserted_fields() {
        let diagnostic = duplicate_cell_path();
        let copy = diagnostic.clone();

        assert_eq!(copy.class, diagnostic.class);
        assert_eq!(copy.span, diagnostic.span);
        assert_eq!(copy.path, diagnostic.path);
    }

    #[test]
    fn span_and_path_are_both_optional() {
        let diagnostic = Diagnostic {
            class: DiagClass::WriteDuringCommit,
            span: None,
            path: None,
            message: "host write arrived during a commit".to_string(),
        };

        assert_eq!(diagnostic.span, None);
        assert_eq!(diagnostic.path, None);
        assert_eq!(diagnostic.class.id(), "write_during_commit");
    }
}
