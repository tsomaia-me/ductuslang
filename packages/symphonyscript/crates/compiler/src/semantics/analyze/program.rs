use symphonyscript_core::Vocabulary;

use super::internal_error::internal_error;
use crate::semantics::Internal;
use crate::semantics::analysis::Engine;
use crate::semantics::collection::{Variants, collect};
use crate::semantics::ir::Hir;
use crate::semantics::reporting::Report;
use crate::semantics::scopes::check_scopes;

pub(crate) fn program<R>(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    report: &mut Report,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> Option<R> {
    match analyzed(hir, vocabulary, variants, report, inspect) {
        Ok(found) => Some(found),
        Err(internal) => {
            internal_error(hir, report, internal);
            None
        }
    }
}

fn analyzed<R>(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    report: &mut Report,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> Result<R, Internal> {
    let collected = collect(hir, vocabulary, variants, report)?;
    check_scopes(hir, &collected, vocabulary, variants, false, report)?;
    let mut engine = Engine::new(hir, vocabulary, &collected, variants);
    let outcome = engine.analyze();
    report.diagnostics.append(&mut engine.report.diagnostics);
    outcome?;
    Ok(inspect(&engine))
}
