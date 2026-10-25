use super::span::span;
use crate::diagnostics::{Code, Diagnostic, Related};
use crate::semantics::ir::{Hir, Loc};

#[derive(Default)]
pub(crate) struct Report {
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn error(&mut self, hir: &Hir, code: Code, loc: Loc, summary: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(code, span(hir, loc), summary));
    }

    pub fn error_with(
        &mut self,
        hir: &Hir,
        code: Code,
        loc: Loc,
        related: &[(Loc, String)],
        summary: impl Into<String>,
    ) {
        let diagnostic =
            Diagnostic::new(code, span(hir, loc), summary).with_related(labelled(hir, related));
        self.diagnostics.push(diagnostic);
    }

    pub fn hinted(
        &mut self,
        hir: &Hir,
        code: Code,
        loc: Loc,
        related: &[(Loc, String)],
        summary: impl Into<String>,
        help: impl Into<String>,
    ) {
        let diagnostic = Diagnostic::new(code, span(hir, loc), summary)
            .with_help(help)
            .with_related(labelled(hir, related));
        self.diagnostics.push(diagnostic);
    }

    pub fn finish(mut self, files: &[String]) -> Vec<Diagnostic> {
        self.diagnostics
            .sort_by_cached_key(|diagnostic| order(files, diagnostic));
        self.diagnostics.dedup();
        self.diagnostics
    }
}

fn labelled(hir: &Hir, related: &[(Loc, String)]) -> Vec<Related> {
    related
        .iter()
        .map(|(loc, label)| Related {
            span: span(hir, *loc),
            label: label.clone(),
        })
        .collect()
}

type OrderKey = (
    usize,
    u32,
    u32,
    Code,
    String,
    Vec<(String, u32, u32, String)>,
);

fn order(files: &[String], diagnostic: &Diagnostic) -> OrderKey {
    let file = files
        .iter()
        .position(|file| *file == diagnostic.span.file)
        .unwrap_or(files.len());
    (
        file,
        diagnostic.span.range.start,
        diagnostic.span.range.end,
        diagnostic.code,
        diagnostic.summary.clone(),
        diagnostic
            .related
            .iter()
            .map(|related| {
                (
                    related.span.file.clone(),
                    related.span.range.start,
                    related.span.range.end,
                    related.label.clone(),
                )
            })
            .collect(),
    )
}
