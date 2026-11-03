use symphonyscript_core::{Limits, Vocabulary};

use super::core::{Core, Stage};
use super::coverage::{covers, declines};
use super::declined::Declined;
use super::reason::Reason;
use super::run::Run;
use super::stages::Stages;
use crate::semantics::Internal;
use crate::semantics::collection::Variants;
use crate::semantics::ir::flat::FlatIr;

pub(crate) struct Inputs<'a> {
    pub prelude: &'a FlatIr,
    pub ir: &'a FlatIr,
    pub vocabulary: &'a Vocabulary,
    pub variants: &'a Variants,
    pub limits: Limits,
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
    let mut stages = Stages::begin(ir.op.len());
    let mut core = Core::new(ir, inputs.vocabulary, inputs.variants, inputs.limits);
    core.front_end_drawn()?;
    if let Err(reason) = stage(core.collect())? {
        return declined(reason);
    }
    stages.mark("collect");
    if core.stop.is_some() {
        return Ok(Some(finished(core)));
    }
    if let Err(reason) = stage(core.expand())? {
        return declined(reason);
    }
    stages.mark("expand");
    core.declaration_cycles()?;
    if let Err(reason) = stage(core.type_cells())? {
        return declined(reason);
    }
    stages.mark("resolve");
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
    if !core.unknown_kind {
        core.check_reachability()?;
    }
    core.consistency()?;
    core.unread_named_faults()?;
    stages.mark("infer and check");
    Ok(Some(finished(core)))
}

fn finished(mut core: Core<'_>) -> Run {
    if let Some((at, diagnostic)) = core.stop.take() {
        core.diagnostics.truncate(at);
        core.diagnostics.push(diagnostic);
    }
    let mut seen = std::collections::HashSet::new();
    core.diagnostics
        .retain(|diagnostic| seen.insert(format!("{diagnostic:?}")));
    Run {
        diagnostics: core.diagnostics,
        placeholders: core.placeholders,
    }
}
