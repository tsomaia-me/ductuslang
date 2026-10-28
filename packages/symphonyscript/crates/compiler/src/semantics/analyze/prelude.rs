use symphonyscript_core::{Limits, Vocabulary};

use super::internal_error::internal_error;
use crate::diagnostics::Severity;
use crate::semantics::Internal;
use crate::semantics::analysis::Engine;
use crate::semantics::collection::{Variants, collect};
use crate::semantics::ir::Hir;
use crate::semantics::reporting::Report;
use crate::semantics::scopes::check_scopes;

pub(crate) fn prelude(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    limits: Limits,
    report: &mut Report,
) {
    if let Err(internal) = resolved(hir, vocabulary, variants, limits, report) {
        internal_error(hir, report, internal);
    }
}

fn resolved(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    limits: Limits,
    report: &mut Report,
) -> Result<(), Internal> {
    let collected = collect(hir, vocabulary, variants, report)?;
    check_scopes(hir, &collected, vocabulary, variants, true, report)?;
    if report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity() == Severity::Error)
    {
        return Ok(());
    }
    let mut engine = Engine::new(hir, vocabulary, &collected, variants, limits);
    let outcome = engine
        .resolve_prelude_lets()
        .and_then(|()| engine.draw_declaration_cycles());
    engine.flush_diagnostics();
    report.diagnostics.append(&mut engine.report.diagnostics);
    outcome
}
