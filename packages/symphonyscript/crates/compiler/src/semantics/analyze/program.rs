use symphonyscript_core::Vocabulary;

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
) -> R {
    let collected = collect(hir, vocabulary, variants, report);
    check_scopes(hir, &collected, vocabulary, variants, false, report);
    let mut engine = Engine::new(hir, vocabulary, &collected, variants);
    engine.expand();
    engine.infer();
    engine.check_program();
    engine.flush_diagnostics();
    report.diagnostics.append(&mut engine.report.diagnostics);
    inspect(&engine)
}
