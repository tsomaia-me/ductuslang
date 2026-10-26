use crate::semantics::Internal;
use crate::semantics::analysis::assumed::Assumed;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::resolve::LayoutFound;
use crate::semantics::collection::strongly_connected;
use crate::semantics::ir::StateId;

impl Engine<'_> {
    pub(super) fn settle_states(&mut self) -> Result<(), Internal> {
        let mut members: Vec<BindingId> = Vec::new();
        for index in 0..self.hir.states.len() {
            let state = StateId(u32::try_from(index).map_err(|_| Internal::missed("state"))?);
            let Some(BoundVal::Scalar(Bound::Cell(binding))) = self.memo.states.get(&state) else {
                continue;
            };
            let binding = *binding;
            if self.memo.settled.contains(&binding) {
                continue;
            }
            if self.memo.binding_layouts.contains_key(&binding) {
                members.push(binding);
            } else {
                self.memo.settled.insert(binding);
            }
        }
        let positions: FastMap<BindingId, usize> = members
            .iter()
            .enumerate()
            .map(|(position, binding)| (*binding, position))
            .collect();
        let mut edges = Vec::with_capacity(members.len());
        for binding in &members {
            let init = self.binding(*binding)?.init;
            let mut targets: Vec<usize> = self
                .state_dependencies(init)?
                .into_iter()
                .filter_map(|dependency| positions.get(&dependency).copied())
                .collect();
            targets.sort_unstable();
            targets.dedup();
            edges.push(targets);
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
            self.settle_group(&group, &readers)?;
        }
        self.memo.layouts.retain(|_, found| found.certain);
        Ok(())
    }

    fn settle_group(
        &mut self,
        group: &[BindingId],
        readers: &[Vec<usize>],
    ) -> Result<(), Internal> {
        let fallbacks = self.group_fallbacks(group)?;
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
            let found = self.assumed_layout(init, &assumed)?;
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
        Ok(())
    }

    fn group_fallbacks(
        &mut self,
        group: &[BindingId],
    ) -> Result<FastMap<BindingId, Vec<Option<Layout>>>, Internal> {
        let mut fallbacks: FastMap<BindingId, Vec<Option<Layout>>> = FastMap::default();
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
                    } => self.known_layout(fallback)?.shaped.ok().flatten(),
                    _ => None,
                };
                fallbacks.entry(target).or_default().push(layout);
            }
        }
        Ok(fallbacks)
    }

    fn assumed_layout(&mut self, node: NodeId, assumed: &Assumed) -> Result<LayoutFound, Internal> {
        let mut previous = None;
        loop {
            let mut need = Vec::new();
            if let Some(found) = self.layout_in(node, Some(assumed), &mut need)? {
                return Ok(found);
            }
            if need.is_empty() || previous.as_ref() == Some(&need) {
                return Err(Internal::missed("array layout"));
            }
            for query in &need {
                self.demand(*query)?;
            }
            previous = Some(need);
        }
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
