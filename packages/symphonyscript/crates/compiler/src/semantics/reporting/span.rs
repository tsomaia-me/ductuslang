use symphonyscript_core::Span;

use crate::semantics::ir::{Hir, Loc};

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
