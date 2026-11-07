use symphonyscript_core::{Limits, Vocabulary};

use super::core::{Core, Stage};
use super::coverage::{covers, declines};
use super::declined::Declined;
use super::reason::Reason;
use super::run::Run;
use super::stages::Stages;
use crate::diagnostics::Diagnostic;
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
    let desugared = super::dot_calls::dot_calls(inputs.ir);
    let (ir, dotted) = match &desugared {
        Some((owned, calls)) => (owned, calls.as_slice()),
        None => (inputs.ir, &[][..]),
    };
    let looped = super::bundle_fors::bundle_fors(ir);
    let (ir, loops) = match &looped {
        Some((owned, loops)) => (owned, loops.as_slice()),
        None => (ir, &[][..]),
    };
    if !covers(ir) {
        return Ok(None);
    }
    if !inputs.prelude.top.is_empty() {
        return declined(Reason::PreludeItems);
    }
    let mut stages = Stages::begin(ir.op.len());
    let mut core = Core::new(ir, inputs.vocabulary, inputs.variants, inputs.limits);
    core.dot_calls = dotted.iter().copied().collect();
    core.bundle_loops = loops.iter().copied().collect();
    core.front_end_drawn()?;
    if let Err(reason) = stage(core.collect())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.check_variants())? {
        return declined(reason);
    }
    core.destination_names()?;
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
    core.bare_declarations()?;
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
    if let Err(reason) = stage(core.check_guards())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.check_reactive_lets())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.check_cells())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.check_unread_arguments())? {
        return declined(reason);
    }
    if let Err(reason) = stage(core.check_cyclic_roots())? {
        return declined(reason);
    }
    core.uncalled_not_values()?;
    core.uncalled_not_callables()?;
    core.settle_capacities()?;
    core.check_integer_cells()?;
    if let Err(reason) = stage(core.plain_in_family())? {
        return declined(reason);
    }
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
    let key = |diagnostic: &Diagnostic| {
        (
            diagnostic.code,
            diagnostic.span.range.start,
            diagnostic.span.range.end,
            diagnostic
                .related
                .last()
                .map(|related| related.span.range.start),
        )
    };
    let mut order: Vec<(_, usize)> = core
        .diagnostics
        .iter()
        .enumerate()
        .map(|(at, diagnostic)| (key(diagnostic), at))
        .collect();
    order.sort_unstable();
    let mut kept = vec![false; core.diagnostics.len()];
    let mut group: Vec<usize> = Vec::new();
    let mut group_key = None;
    for (held, at) in order {
        if group_key != Some(held) {
            group.clear();
            group_key = Some(held);
        }
        let diagnostic = core.diagnostics.get(at);
        if group
            .iter()
            .any(|earlier| core.diagnostics.get(*earlier) == diagnostic)
        {
            continue;
        }
        group.push(at);
        if let Some(keep) = kept.get_mut(at) {
            *keep = true;
        }
    }
    let mut diagnostics = std::mem::take(&mut core.diagnostics);
    let mut at = 0;
    diagnostics.retain(|_| {
        let keep = kept.get(at).copied().unwrap_or(false);
        at += 1;
        keep
    });
    Run {
        diagnostics,
        placeholders: core.placeholders,
    }
}
