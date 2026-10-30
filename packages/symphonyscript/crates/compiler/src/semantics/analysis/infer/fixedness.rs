use crate::semantics::Internal;
use crate::semantics::analysis::answers::CellRead;
use crate::semantics::analysis::assumed::Assumed;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::settling::Settling;
use crate::semantics::collection::strongly_connected;
use crate::semantics::ir::StateId;

type Fallbacks = FastMap<BindingId, Vec<Option<Layout>>>;

impl Engine<'_> {
    pub(super) fn settle_states(&mut self) -> Result<(), Internal> {
        for index in 0..self.hir.states.len() {
            crate::semantics::counted_walk::CountedWalk::FixednessSettleStates.step();
            let state = StateId(u32::try_from(index).map_err(|_| Internal::missed("state"))?);
            let mut previous = None;
            loop {
                let mut need = Vec::new();
                if self.settle_from(state, &mut need)? != Settling::Pending {
                    break;
                }
                if need.is_empty() || previous.as_ref() == Some(&need) {
                    return Err(Internal::missed("state settlement"));
                }
                for query in &need {
                    self.demand(*query)?;
                }
                previous = Some(need);
            }
        }
        Ok(())
    }

    pub(in crate::semantics::analysis) fn settle_from(
        &mut self,
        state: StateId,
        need: &mut Vec<Query>,
    ) -> Result<Settling, Internal> {
        let mut reached = vec![state];
        let mut seen: FastSet<StateId> = FastSet::default();
        seen.insert(state);
        let mut unresolved = Vec::new();
        let mut members: Vec<BindingId> = Vec::new();
        let mut reads: Vec<Vec<StateId>> = Vec::new();
        let mut unbound = Vec::new();
        let mut visited = Vec::new();
        while let Some(current) = reached.pop() {
            self.memo.settlement_visits += 1;
            visited.push(current);
            if let Some(witness) = self.blocking_witness(current) {
                self.record_blocked(&visited, witness);
                return Ok(Settling::Blocked);
            }
            if !self.memo.states.contains_key(&current) {
                unresolved.push(current);
                continue;
            }
            let Some(binding) = self.state_binding(current) else {
                continue;
            };
            if self.memo.settled.contains(&binding) {
                continue;
            }
            if !self.memo.binding_layouts.contains_key(&binding) {
                self.memo.settled.insert(binding);
                continue;
            }
            let targets = self.state_reads(self.binding(binding)?.init, &mut unbound)?;
            for target in &targets {
                if seen.insert(*target) {
                    reached.push(*target);
                }
            }
            members.push(binding);
            reads.push(targets);
        }
        if !unbound.is_empty() {
            if self.declaring() {
                return Ok(Settling::Blocked);
            }
            need.extend(unbound);
            return Ok(Settling::Pending);
        }
        if !unresolved.is_empty() {
            let declaring = self.declaring();
            let witness = unresolved
                .iter()
                .copied()
                .find(|state| declaring || self.in_progress(Query::State(*state)));
            if let Some(witness) = witness {
                self.record_blocked(&visited, witness);
                return Ok(Settling::Blocked);
            }
            need.extend(unresolved.into_iter().map(Query::State));
            return Ok(Settling::Pending);
        }
        let positions: FastMap<BindingId, usize> = members
            .iter()
            .enumerate()
            .map(|(position, binding)| (*binding, position))
            .collect();
        let mut edges = Vec::with_capacity(members.len());
        for targets in &reads {
            let mut local: Vec<usize> = targets
                .iter()
                .filter_map(|target| self.state_binding(*target))
                .filter_map(|binding| positions.get(&binding).copied())
                .collect();
            local.sort_unstable();
            local.dedup();
            edges.push(local);
        }
        for component in strongly_connected(&edges)? {
            let group: Vec<BindingId> = component
                .iter()
                .map(|position| {
                    members
                        .get(*position)
                        .copied()
                        .ok_or(Internal::missed("state"))
                })
                .collect::<Result<_, _>>()?;
            let readers = readers_within(&group, &component, &edges)?;
            if !self.settle_group(&group, &readers, need)? {
                return Ok(Settling::Pending);
            }
        }
        Ok(Settling::Settled)
    }

    fn blocking_witness(&self, state: StateId) -> Option<StateId> {
        let (epoch, witness) = self.memo.blocked_walks.get(&state).copied()?;
        let blocks = epoch == self.memo.settle_epoch
            && !self.memo.states.contains_key(&witness)
            && (self.declaring() || self.in_progress(Query::State(witness)));
        blocks.then_some(witness)
    }

    fn record_blocked(&mut self, visited: &[StateId], witness: StateId) {
        let epoch = self.memo.settle_epoch;
        for state in visited {
            self.memo.blocked_walks.insert(*state, (epoch, witness));
        }
    }

    fn state_reads(
        &mut self,
        init: NodeId,
        unbound: &mut Vec<Query>,
    ) -> Result<Vec<StateId>, Internal> {
        let mut computed: FastMap<BindingId, (Vec<StateId>, bool)> = FastMap::default();
        loop {
            let (found, _, missing) = self.scan_state_reads(init, &computed, unbound)?;
            if missing.is_empty() {
                return Ok(found);
            }
            self.settle_binding_reads(missing, &mut computed, unbound)?;
        }
    }

    fn settle_binding_reads(
        &mut self,
        mut work: Vec<BindingId>,
        computed: &mut FastMap<BindingId, (Vec<StateId>, bool)>,
        unbound: &mut Vec<Query>,
    ) -> Result<(), Internal> {
        let mut active: FastSet<BindingId> = work.iter().copied().collect();
        while let Some(&binding) = work.last() {
            crate::semantics::counted_walk::CountedWalk::FixednessSettleBindingReads.step();
            if computed.contains_key(&binding)
                || self.memo.binding_state_reads.contains_key(&binding)
            {
                work.pop();
                continue;
            }
            let init = self.binding(binding)?.init;
            let (found, clean, missing) = self.scan_state_reads(init, computed, unbound)?;
            let cyclic = missing.iter().any(|child| active.contains(child));
            let waiting: Vec<BindingId> = missing
                .into_iter()
                .filter(|child| !active.contains(child))
                .collect();
            if !waiting.is_empty() {
                for child in waiting {
                    active.insert(child);
                    work.push(child);
                }
                continue;
            }
            work.pop();
            active.remove(&binding);
            let clean = clean && !cyclic;
            if clean {
                self.memo
                    .binding_state_reads
                    .insert(binding, found.clone().into());
            }
            computed.insert(binding, (found, clean));
        }
        Ok(())
    }

    fn scan_state_reads(
        &mut self,
        init: NodeId,
        computed: &FastMap<BindingId, (Vec<StateId>, bool)>,
        unbound: &mut Vec<Query>,
    ) -> Result<(Vec<StateId>, bool, Vec<BindingId>), Internal> {
        let mut found = Vec::new();
        let mut clean = true;
        let mut missing = Vec::new();
        let mut cells = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![init];
        while let Some(node) = stack.pop() {
            self.memo.settlement_visits += 1;
            if !seen.insert(node) {
                continue;
            }
            match self.cell_read(node)? {
                CellRead::Binding(binding) => match self.binding(binding)?.kind {
                    BindingKind::State { state } => found.push(state),
                    BindingKind::Input { .. }
                    | BindingKind::Let
                    | BindingKind::Param
                    | BindingKind::Attr { .. } => cells.push(binding),
                },
                CellRead::History { cell, fallback } => {
                    match cell {
                        HistoryCell::State(state) => found.push(state),
                        HistoryCell::Let(id) if self.let_bound(id).is_none() => {
                            clean = false;
                            unbound.push(self.history_cell_query(cell));
                        }
                        HistoryCell::Input(_) | HistoryCell::Let(_) => {
                            cells.extend(self.history_bindings(node)?);
                        }
                    }
                    stack.extend(fallback);
                }
                CellRead::Named(inner) => stack.push(inner),
                CellRead::Operands(operands) => stack.extend(operands),
            }
        }
        for binding in cells {
            if let Some(known) = self.memo.binding_state_reads.get(&binding) {
                found.extend(known.iter().copied());
            } else if let Some((known, known_clean)) = computed.get(&binding) {
                found.extend(known.iter().copied());
                clean &= *known_clean;
            } else {
                missing.push(binding);
            }
        }
        Ok((found, clean, missing))
    }

    fn settle_group(
        &mut self,
        group: &[BindingId],
        readers: &[Vec<usize>],
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let Some(fallbacks) = self.group_fallbacks(group, need)? else {
            return Ok(false);
        };
        let mut assumed = Assumed::default();
        for binding in group {
            assumed.members.insert(*binding, false);
        }
        let mut layouts: Vec<Option<Layout>> = vec![None; group.len()];
        let mut pending: Vec<usize> = (0..group.len()).rev().collect();
        while let Some(position) = pending.pop() {
            crate::semantics::counted_walk::CountedWalk::FixednessSettleGroup.step();
            let binding = *group.get(position).ok_or(Internal::missed("state"))?;
            if assumed.members.get(&binding) == Some(&true) {
                continue;
            }
            let init = self.binding(binding)?.init;
            let Some(found) = self.layout_in(init, Some(&assumed), need)? else {
                return Ok(false);
            };
            let layout = found.shaped.ok().flatten();
            let own = fallbacks
                .get(&binding)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let fixed = layout.as_ref().is_some_and(|layout| {
                !layout.variable && own.iter().all(|fallback| fallback.as_ref() == Some(layout))
            });
            if let Some(slot) = layouts.get_mut(position) {
                *slot = layout;
            }
            if !fixed {
                assumed.members.insert(binding, true);
                for reader in readers.get(position).into_iter().flatten() {
                    pending.push(*reader);
                }
            }
        }
        for (position, binding) in group.iter().enumerate() {
            let layout = layouts
                .get(position)
                .cloned()
                .flatten()
                .or_else(|| self.memo.binding_layouts.get(binding).cloned());
            let variable = assumed.members.get(binding) == Some(&true);
            if let Some(layout) = layout {
                let settled = if variable {
                    Layout::variable(layout.inner().to_vec())
                } else {
                    layout
                };
                self.memo.binding_layouts.insert(*binding, settled);
            }
            self.memo.settled.insert(*binding);
        }
        self.memo.settle_epoch += 1;
        Ok(true)
    }

    fn group_fallbacks(
        &mut self,
        group: &[BindingId],
        need: &mut Vec<Query>,
    ) -> Result<Option<Fallbacks>, Internal> {
        let unassumed = Assumed::default();
        let mut fallbacks: Fallbacks = FastMap::default();
        let mut missing = false;
        for binding in group {
            let init = self.binding(*binding)?.init;
            for (node, cell) in self.history_reads(init)? {
                let HistoryCell::State(state) = cell else {
                    continue;
                };
                let Some(target) = self.state_binding(state) else {
                    continue;
                };
                if !group.contains(&target) {
                    continue;
                }
                let layout = match self.node(node)?.op {
                    Op::History {
                        fallback: Some(fallback),
                        ..
                    } => match self.layout_in(fallback, Some(&unassumed), need)? {
                        Some(found) => found.shaped.ok().flatten(),
                        None => {
                            missing = true;
                            continue;
                        }
                    },
                    _ => None,
                };
                fallbacks.entry(target).or_default().push(layout);
            }
        }
        Ok((!missing).then_some(fallbacks))
    }
}

fn readers_within(
    group: &[BindingId],
    component: &[usize],
    edges: &[Vec<usize>],
) -> Result<Vec<Vec<usize>>, Internal> {
    let local: FastMap<usize, usize> = component
        .iter()
        .enumerate()
        .map(|(position, member)| (*member, position))
        .collect();
    let mut readers = vec![Vec::new(); group.len()];
    for (reader, member) in component.iter().enumerate() {
        for target in edges.get(*member).ok_or(Internal::missed("state"))? {
            if let Some(position) = local.get(target)
                && let Some(list) = readers.get_mut(*position)
            {
                list.push(reader);
            }
        }
    }
    Ok(readers)
}
