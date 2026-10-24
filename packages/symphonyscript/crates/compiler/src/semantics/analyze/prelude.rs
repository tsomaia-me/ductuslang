use symphonyscript_core::Vocabulary;

use crate::semantics::collection::{Variants, collect};
use crate::semantics::ir::Hir;
use crate::semantics::reporting::Report;
use crate::semantics::scopes::check_scopes;

pub(crate) fn prelude(
    hir: &Hir,
    vocabulary: &Vocabulary,
    variants: &Variants,
    report: &mut Report,
) {
    let collected = collect(hir, vocabulary, variants, report);
    check_scopes(hir, &collected, vocabulary, variants, true, report);
}
