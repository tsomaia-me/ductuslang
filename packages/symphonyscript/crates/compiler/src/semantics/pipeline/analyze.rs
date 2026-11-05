use symphonyscript_core::{Limits, Vocabulary};

use super::core::{Core, Stage};
use super::coverage::{covers, declines};
use super::declined::Declined;
use super::reason::Reason;
use super::run::Run;
use super::stages::Stages;
use crate::diagnostics::Code;
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
    if core.stop.is_some() {
        return Ok(Some(finished(core)));
    }
    if let Err(reason) = stage(core.unused_patterns())? {
        return declined(reason);
    }
    core.resolve_destinations()?;
    if let Err(reason) = stage(core.resolve_paths())? {
        return declined(reason);
    }
    core.declaration_cycles()?;
    if let Err(reason) = stage(core.type_cells())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.key_deferred_uses())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.type_attr_reads())? {
        return declined(reason);
    }
    core.share_ref_bodies()?;
    if let Err(reason) = stage(core.choose_destinations())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.resolve_chosen_paths())? {
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
    core.unreached_body_path_faults();
    core.replay_use_faults();
    stages.mark("infer and check");
    Ok(Some(finished(core)))
}

fn finished(mut core: Core<'_>) -> Run {
    if let Some((at, diagnostic)) = core.stop.take() {
        core.diagnostics.truncate(at);
        core.diagnostics.push(diagnostic);
    }
    let mut seen: std::collections::HashMap<(Code, u32, u32, Option<u32>), Vec<usize>> =
        std::collections::HashMap::new();
    let mut kept = Vec::new();
    for (at, diagnostic) in core.diagnostics.iter().enumerate() {
        let same = seen
            .entry((
                diagnostic.code,
                diagnostic.span.range.start,
                diagnostic.span.range.end,
                diagnostic
                    .related
                    .last()
                    .map(|related| related.span.range.start),
            ))
            .or_default();
        if same
            .iter()
            .any(|earlier| core.diagnostics.get(*earlier) == Some(diagnostic))
        {
            continue;
        }
        same.push(at);
        kept.push(at);
    }
    let mut diagnostics = std::mem::take(&mut core.diagnostics);
    let mut at = 0;
    diagnostics.retain(|_| {
        let keep = kept.binary_search(&at).is_ok();
        at += 1;
        keep
    });
    Run {
        diagnostics,
        placeholders: core.placeholders,
    }
}
