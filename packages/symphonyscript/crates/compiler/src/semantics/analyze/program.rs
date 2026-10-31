use symphonyscript_core::{Limits, Vocabulary};

use super::internal_error::internal_error;
use super::unresolved::Unresolved;
use crate::semantics::Internal;
use crate::semantics::analysis::Engine;
use crate::semantics::collection::{Variants, collect};
use crate::semantics::ir::{Hir, LetId};
use crate::semantics::reporting::Report;
use crate::semantics::scopes::check_scopes;

pub(crate) fn program<R>(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    limits: Limits,
    demanded: &[LetId],
    report: &mut Report,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> (Option<R>, Unresolved) {
    let mut unresolved = Unresolved {
        names: Vec::new(),
        computed_lets: Vec::new(),
    };
    match analyzed(
        hir,
        (vocabulary, variants, limits),
        (demanded, report),
        &mut unresolved,
        inspect,
    ) {
        Ok(found) => (Some(found), unresolved),
        Err(internal) => {
            internal_error(hir, report, internal);
            (
                None,
                Unresolved {
                    names: Vec::new(),
                    computed_lets: Vec::new(),
                },
            )
        }
    }
}

fn analyzed<R>(
    hir: &Hir,
    (vocabulary, variants, limits): (&Vocabulary, &Variants, Limits),
    (demanded, report): (&[LetId], &mut Report),
    unresolved: &mut Unresolved,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> Result<R, Internal> {
    let collected = collect(hir, vocabulary, variants, report)?;
    unresolved.names = check_scopes(hir, &collected, vocabulary, variants, false, report)?;
    let mut engine = Engine::new(hir, vocabulary, &collected, variants, limits);
    engine.demanded_lets = demanded.iter().copied().collect();
    let outcome = engine.analyze();
    report.diagnostics.append(&mut engine.report.diagnostics);
    outcome?;
    if !unresolved.names.is_empty() {
        unresolved.computed_lets = engine.computed_lets();
    }
    Ok(inspect(&engine))
}
