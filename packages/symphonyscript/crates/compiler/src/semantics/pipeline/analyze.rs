use symphonyscript_core::{Limits, Vocabulary};

use super::core::Core;
use super::coverage::covers;
use super::run::Run;
use crate::semantics::Internal;
use crate::semantics::collection::Variants;
use crate::semantics::ir::flat::FlatIr;

pub(crate) fn analyze(
    prelude: &FlatIr,
    ir: &FlatIr,
    vocabulary: &Vocabulary,
    variants: &Variants,
    limits: Limits,
) -> Result<Option<Run>, Internal> {
    if !covers(prelude, ir) {
        return Ok(None);
    }
    let mut core = Core::new(ir, vocabulary, variants, limits);
    if core.collect()?.is_err() {
        return Ok(None);
    }
    if core.expand()?.is_err() {
        return Ok(None);
    }
    core.declaration_cycles()?;
    if core.check_attributes()?.is_err() {
        return Ok(None);
    }
    core.check_counts()?;
    core.check_reachability()?;
    Ok(Some(Run {
        diagnostics: core.diagnostics,
        placeholders: core.placeholders,
    }))
}
