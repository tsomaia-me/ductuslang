use symphonyscript_core::Vocabulary;

use super::internal_error::internal_error;
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
    let checked = collect(hir, vocabulary, variants, report)
        .and_then(|collected| check_scopes(hir, &collected, vocabulary, variants, true, report));
    if let Err(internal) = checked {
        internal_error(hir, report, internal);
    }
}
