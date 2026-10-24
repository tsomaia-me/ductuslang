use symphonyscript_core::Vocabulary;

use super::analysis::Engine;
use super::collection::collect;
use super::ir::Hir;
use super::reporting::Report;
use super::scopes::check_scopes;

pub(crate) fn prelude(hir: &Hir, vocabulary: &Vocabulary, report: &mut Report) {
    let collected = collect(hir, vocabulary, report);
    check_scopes(hir, &collected, vocabulary, true, report);
}

pub(crate) fn program<R>(
    hir: &Hir,
    vocabulary: &Vocabulary,
    report: &mut Report,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> R {
    let collected = collect(hir, vocabulary, report);
    check_scopes(hir, &collected, vocabulary, false, report);
    let mut engine = Engine::new(hir, vocabulary, &collected);
    engine.expand();
    engine.infer();
    engine.check_program();
    report.diagnostics.append(&mut engine.report.diagnostics);
    inspect(&engine)
}
