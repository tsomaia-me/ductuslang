use symphonyscript_core::Span;

use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::ir::{Hir, Loc};

#[derive(Default)]
pub(crate) struct Report {
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn error(&mut self, hir: &Hir, code: Code, loc: Loc, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(code, span(hir, loc), message));
    }

    pub fn error_with(
        &mut self,
        hir: &Hir,
        code: Code,
        loc: Loc,
        secondary: &[Loc],
        message: impl Into<String>,
    ) {
        let mut diagnostic = Diagnostic::new(code, span(hir, loc), message);
        diagnostic.secondary = secondary.iter().map(|other| span(hir, *other)).collect();
        self.diagnostics.push(diagnostic);
    }

    pub fn hinted(
        &mut self,
        hir: &Hir,
        code: Code,
        loc: Loc,
        secondary: &[Loc],
        message: impl Into<String>,
        hint: impl Into<String>,
    ) {
        let mut diagnostic = Diagnostic::new(code, span(hir, loc), message).with_hint(hint);
        diagnostic.secondary = secondary.iter().map(|other| span(hir, *other)).collect();
        self.diagnostics.push(diagnostic);
    }

    pub fn finish(mut self) -> Vec<Diagnostic> {
        self.diagnostics.sort_by_cached_key(order);
        self.diagnostics.dedup();
        self.diagnostics
    }
}

type OrderKey = (String, u32, u32, Code, String, Vec<(String, u32, u32)>);

fn order(diagnostic: &Diagnostic) -> OrderKey {
    (
        diagnostic.span.file.clone(),
        diagnostic.span.range.start,
        diagnostic.span.range.end,
        diagnostic.code,
        diagnostic.message.clone(),
        diagnostic
            .secondary
            .iter()
            .map(|span| (span.file.clone(), span.range.start, span.range.end))
            .collect(),
    )
}

pub(crate) fn span(hir: &Hir, loc: Loc) -> Span {
    Span {
        file: hir
            .files
            .get(loc.file as usize)
            .cloned()
            .unwrap_or_default(),
        range: loc.start..loc.end,
    }
}
