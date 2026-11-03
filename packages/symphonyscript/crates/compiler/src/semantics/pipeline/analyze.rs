use symphonyscript_core::{Limits, Vocabulary};

use super::core::{Core, Stage};
use super::coverage::{covers, declines};
use super::declined::Declined;
use super::reason::Reason;
use super::run::Run;
use crate::semantics::Internal;
use crate::semantics::collection::Variants;
use crate::semantics::ir::flat::FlatIr;

pub(crate) struct Inputs<'a> {
    pub prelude: &'a FlatIr,
    pub ir: &'a FlatIr,
    pub vocabulary: &'a Vocabulary,
    pub variants: &'a Variants,
    pub limits: Limits,
    pub front_end_faults: bool,
}

fn declined(reason: Reason) -> Result<Option<Run>, Internal> {
    if declines(reason) {
        Ok(None)
    } else {
        Err(Internal::missed(
            "a decline whose row the coverage table lands",
        ))
    }
}

fn stage<T>(outcome: Stage<T>) -> Result<Result<T, Reason>, Internal> {
    Ok(outcome?.map_err(|Declined(reason)| reason))
}

pub(crate) fn analyze(inputs: &Inputs<'_>) -> Result<Option<Run>, Internal> {
    let ir = inputs.ir;
    if !covers(ir) {
        return Ok(None);
    }
    if !inputs.prelude.top.is_empty() {
        return declined(Reason::PreludeItems);
    }
    let faulty = inputs.front_end_faults
        || !ir.drawn.is_empty()
        || !ir.regions.is_empty()
        || ir.names.iter().any(|name| name.faulty);
    if faulty {
        return declined(Reason::FrontEndFault);
    }
    let mut core = Core::new(ir, inputs.vocabulary, inputs.variants, inputs.limits);
    if let Err(reason) = stage(core.collect())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.expand())? {
        return declined(reason);
    }
    core.declaration_cycles()?;
    if let Err(reason) = stage(core.type_cells())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.check_attributes())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.check_cells())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.check_reactive_lets())? {
        return declined(reason);
    }
    core.check_integer_cells()?;
    core.check_counts()?;
    core.check_reachability()?;
    core.consistency()?;
    Ok(Some(Run {
        diagnostics: core.diagnostics,
        placeholders: core.placeholders,
    }))
}
