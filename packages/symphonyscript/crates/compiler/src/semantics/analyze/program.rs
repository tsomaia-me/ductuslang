use symphonyscript_core::{Limits, Vocabulary};

use super::internal_error::internal_error;
use crate::semantics::Internal;
use crate::semantics::analysis::Engine;
use crate::semantics::collection::{Variants, collect};
use crate::semantics::ir::{ExprId, Hir};
use crate::semantics::reporting::Report;
use crate::semantics::scopes::check_scopes;

pub(crate) fn program<R>(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    limits: Limits,
    report: &mut Report,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> (Option<R>, Vec<ExprId>) {
    let mut placeholders = Vec::new();
    match analyzed(
        hir,
        (vocabulary, variants, limits),
        report,
        &mut placeholders,
        inspect,
    ) {
        Ok(found) => (Some(found), placeholders),
        Err(internal) => {
            internal_error(hir, report, internal);
            (None, Vec::new())
        }
    }
}

fn analyzed<R>(
    hir: &Hir,
    (vocabulary, variants, limits): (&Vocabulary, &Variants, Limits),
    report: &mut Report,
    placeholders: &mut Vec<ExprId>,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> Result<R, Internal> {
    let collected = collect(hir, vocabulary, variants, report)?;
    *placeholders = check_scopes(hir, &collected, vocabulary, variants, false, report)?;
    let mut engine = Engine::new(hir, vocabulary, &collected, variants, limits);
    let outcome = engine.analyze();
    report.diagnostics.append(&mut engine.report.diagnostics);
    outcome?;
    Ok(inspect(&engine))
}
