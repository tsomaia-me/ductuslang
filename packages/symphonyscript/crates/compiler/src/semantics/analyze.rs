use symphonyscript_core::Vocabulary;

use super::analysis::Engine;
use super::collection::{Variants, collect};
use super::ir::Hir;
use super::reporting::Report;
use super::scopes::check_scopes;

pub(crate) fn prelude(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    report: &mut Report,
) {
    let collected = collect(hir, vocabulary, variants, report);
    check_scopes(hir, &collected, vocabulary, variants, true, report);
}

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
