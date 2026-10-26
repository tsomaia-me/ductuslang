use crate::semantics::Internal;
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
            let targets = self.state_reads(self.binding(binding)?.init)?;
            for target in &targets {
                if seen.insert(*target) {
                    reached.push(*target);
                }
            }
            members.push(binding);
            reads.push(targets);
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

    fn state_reads(&self, init: NodeId) -> Result<Vec<StateId>, Internal> {
        let mut found = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![init];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::Read(binding) => match self.binding(*binding)?.kind {
                    BindingKind::State { state } => found.push(state),
                    BindingKind::Input { .. }
                    | BindingKind::Let
                    | BindingKind::Param
                    | BindingKind::Attr { .. } => stack.push(self.binding(*binding)?.init),
                },
                Op::History { cell, fallback, .. } => {
                    match cell {
                        HistoryCell::State(state) => found.push(*state),
                        HistoryCell::Input(_) | HistoryCell::Let(_) => {
                            for binding in self.history_bindings(node)? {
                                stack.push(self.binding(binding)?.init);
                            }
                        }
                    }
                    stack.extend(fallback.iter().copied());
                }
                op => stack.extend(op.operands()),
            }
        }
        Ok(found)
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
