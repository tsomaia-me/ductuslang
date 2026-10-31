use crate::semantics::Internal;
use crate::semantics::analysis::answers::CellRead;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;
use crate::semantics::collection::strongly_connected;
use crate::semantics::ir::StateId;

impl Engine<'_> {
    pub(super) fn mark_own_history(&mut self) -> Result<(), Internal> {
        for index in 0..self.hir.states.len() {
            crate::semantics::counted_walk::CountedWalk::HistoryReadsMarkOwnHistory.step();
            let state = StateId(u32::try_from(index).unwrap_or(u32::MAX));
            self.mark_own(state)?;
        }
        Ok(())
    }

    fn mark_own(&mut self, state: StateId) -> Result<(), Internal> {
        if self.memo.own_marked.contains(&state) {
            return Ok(());
        }
        let Some(BoundVal::Scalar(Bound::Cell(binding))) = self.memo.states.get(&state).cloned()
        else {
            return Ok(());
        };
        self.memo.own_marked.insert(state);
        let init = self.binding(binding)?.init;
        for (node, cell) in self.history_reads(init)? {
            if cell == HistoryCell::State(state) {
                self.memo.own_history.insert(node);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(in crate::semantics::analysis) fn history_depth(&self, cell: HistoryCell) -> u64 {
        self.nodes
            .iter()
            .filter_map(|node| {
                if let Op::History {
                    cell: read, offset, ..
                } = node.op
                    && read == cell
                {
                    Some(offset)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0)
    }

    pub(in crate::semantics::analysis) fn history_cell_query(&self, cell: HistoryCell) -> Query {
        match cell {
            HistoryCell::Input(input) => Query::Input(input),
            HistoryCell::Let(id) => Query::Let(self.global_of_let(id), LetRef::Top(id)),
            HistoryCell::State(state) => Query::State(state),
        }
    }

    pub(in crate::semantics::analysis) fn own_read(
        &mut self,
        node: NodeId,
        cell: HistoryCell,
    ) -> Result<bool, Internal> {
        let HistoryCell::State(state) = cell else {
            return Ok(false);
        };
        self.mark_own(state)?;
        Ok(self.memo.own_history.contains(&node))
    }

    pub(in crate::semantics::analysis) fn kind_component(
        &mut self,
        start: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<u32>>, Internal> {
        if let Some(found) = self.memo.kind_components.get(&start) {
            return Ok(Some(*found));
        }
        let mut index: FastMap<NodeId, usize> = FastMap::default();
        let mut nodes = Vec::new();
        let mut targets = Vec::new();
        let mut missing = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::HistoryReadsKindComponent.step();
            if index.contains_key(&node) || self.memo.kind_components.contains_key(&node) {
                continue;
            }
            if let Some(query) = self.memo.kind_pending.get(&node).copied()
                && self.bound(query, &mut Vec::new()).is_none()
            {
                missing.push(query);
                continue;
            }
            index.insert(node, nodes.len());
            nodes.push(node);
            let found = self.kind_edges(node, false, &mut missing)?;
            for target in &found {
                if !index.contains_key(target) && !self.memo.kind_components.contains_key(target) {
                    stack.push(*target);
                }
            }
            targets.push(found);
        }
        if let Some(query) = missing.first().copied() {
            for node in nodes {
                self.memo.kind_pending.insert(node, query);
            }
            need.extend(missing);
            return Ok(None);
        }
        let edges: Vec<Vec<usize>> = targets
            .iter()
            .map(|found| {
                found
                    .iter()
                    .filter_map(|target| index.get(target).copied())
                    .collect()
            })
            .collect();
        for component in strongly_connected(&edges)? {
            let looped = match component.as_slice() {
                [only] => edges.get(*only).is_some_and(|out| out.contains(only)),
                _ => true,
            };
            let members: Vec<NodeId> = component
                .iter()
                .filter_map(|position| nodes.get(*position).copied())
                .collect();
            let group = if looped {
                let id = u32::try_from(self.memo.kind_groups.len())
                    .map_err(|_| Internal::missed("kind group"))?;
                self.memo.kind_groups.push(members.clone());
                Some(id)
            } else {
                None
            };
            for member in members {
                self.memo.kind_components.insert(member, group);
            }
        }
        Ok(Some(
            self.memo
                .kind_components
                .get(&start)
                .copied()
                .ok_or(Internal::missed("kind component"))?,
        ))
    }

    fn kind_edges(
        &mut self,
        node: NodeId,
        cut: bool,
        missing: &mut Vec<Query>,
    ) -> Result<Vec<NodeId>, Internal> {
        self.memo.kind_visits += 1;
        self.memo.settlement_visits += 1;
        let mut found = self.node(node)?.op.operands();
        let extra = match self.cell_read(node)? {
            CellRead::Binding(binding) => Some(Ok(binding)),
            CellRead::History { cell, .. } => Some(Err(cell)),
            CellRead::Named(_) | CellRead::Operands(_) => None,
        };
        if let Some(group) = self.memo.groups.get(&node) {
            found.push(*group);
        }
        match extra {
            Some(Ok(binding)) => found.push(self.binding(binding)?.init),
            Some(Err(cell)) if !cut => {
                let query = self.history_cell_query(cell);
                if self.bound(query, missing).is_some() && !self.own_read(node, cell)? {
                    for binding in self.history_bindings(node)? {
                        found.push(self.binding(binding)?.init);
                    }
                }
            }
            Some(Err(_)) | None => {}
        }
        Ok(found)
    }

    pub(in crate::semantics::analysis) fn settle_kinds(
        &mut self,
        group: u32,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let members = self
            .memo
            .kind_groups
            .get(group as usize)
            .cloned()
            .ok_or(Internal::missed("kind group"))?;
        let inside: FastSet<NodeId> = members.iter().copied().collect();
        let mut reads = Vec::new();
        let mut index: FastMap<NodeId, usize> = FastMap::default();
        let mut edges = Vec::with_capacity(members.len());
        let mut outside = Vec::new();
        for (position, node) in members.iter().enumerate() {
            index.insert(*node, position);
            let read = match self.cell_read(*node)? {
                CellRead::History {
                    cell: cell @ (HistoryCell::Let(_) | HistoryCell::State(_)),
                    fallback,
                } => Some((cell, fallback)),
                CellRead::History { .. }
                | CellRead::Binding(_)
                | CellRead::Named(_)
                | CellRead::Operands(_) => None,
            };
            let read = match read {
                Some((cell, fallback)) if !self.own_read(*node, cell)? => Some(fallback),
                Some(_) | None => None,
            };
            let mut ignored = Vec::new();
            let found = self.kind_edges(*node, read.is_some(), &mut ignored)?;
            if let Some(fallback) = read {
                let binding = self
                    .history_bindings(*node)?
                    .first()
                    .copied()
                    .ok_or(Internal::missed("history cell"))?;
                reads.push((*node, binding, fallback));
            }
            outside.extend(
                found
                    .iter()
                    .copied()
                    .filter(|target| !inside.contains(target)),
            );
            edges.push(found);
        }
        let mut missing = Vec::new();
        for node in outside {
            self.sig(node, &mut missing);
        }
        if !missing.is_empty() {
            need.extend(missing);
            return Ok(false);
        }
        let edges: Vec<Vec<usize>> = edges
            .iter()
            .map(|found| {
                found
                    .iter()
                    .filter_map(|target| index.get(target).copied())
                    .collect()
            })
            .collect();
        let mut order = Vec::with_capacity(members.len());
        for component in strongly_connected(&edges)? {
            let [only] = component.as_slice() else {
                return self.unsettled_kinds(&reads);
            };
            if edges.get(*only).is_some_and(|out| out.contains(only)) {
                return self.unsettled_kinds(&reads);
            }
            order.push(*only);
        }
        let mut rank = vec![0; members.len()];
        let mut readers: Vec<Vec<usize>> = vec![Vec::new(); members.len()];
        for (place, position) in order.iter().enumerate() {
            if let Some(slot) = rank.get_mut(*position) {
                *slot = place;
            }
        }
        for (position, out) in edges.iter().enumerate() {
            for target in out {
                if let Some(list) = readers.get_mut(*target) {
                    list.push(position);
                }
            }
        }
        let mut watching: FastMap<usize, Vec<usize>> = FastMap::default();
        let mut positions = Vec::with_capacity(reads.len());
        for (read_index, (read, binding, _)) in reads.iter().enumerate() {
            let init = self.binding(*binding)?.init;
            if let Some(position) = index.get(&init) {
                watching.entry(*position).or_default().push(read_index);
            }
            positions.push(*index.get(read).ok_or(Internal::missed("kind group"))?);
        }
        let saved = Saved {
            deferred: self.deferred.len(),
            matches: self.memo.bare_matches.len(),
            families: self.memo.takes_family.clone(),
            takers: self.memo.template_takers.clone(),
        };
        let mut assumed: Vec<Option<Sig>> = vec![None; reads.len()];
        for (read, ..) in &reads {
            self.memo.history_kinds.insert(*read, None);
        }
        if !self.run_members(&members, &order, need)? {
            return self.abandon_kinds(&members, &reads, saved);
        }
        let mut candidates: Vec<usize> = (0..reads.len()).collect();
        let mut moved = false;
        let limit = 4 * reads.len() + 4;
        for _ in 0..limit {
            let mut changed = Vec::new();
            for read_index in candidates {
                let (Some((read, binding, fallback)), Some(sig)) =
                    (reads.get(read_index), assumed.get(read_index).copied())
                else {
                    continue;
                };
                let init = self.known_sig(self.binding(*binding)?.init)?;
                let cell = self.read_sig(*binding, init)?;
                let before = match (sig, fallback) {
                    (Some(sig), _) => sig.kind,
                    (None, Some(fallback)) => self.known_sig(*fallback)?.kind,
                    (None, None) => cell.kind,
                };
                let kind = match (before, cell.kind) {
                    (Kind::Int, Kind::Float) | (Kind::Float, Kind::Int) => Kind::Float,
                    _ => cell.kind,
                };
                let found = self.known_sig(*read)?;
                if found.kind == kind && found.family == cell.family {
                    continue;
                }
                let next = Some(Sig::of(kind, Width::Adaptive, true, cell.family));
                if let Some(slot) = assumed.get_mut(read_index) {
                    *slot = next;
                }
                self.memo.history_kinds.insert(*read, next);
                changed.extend(positions.get(read_index).copied());
            }
            if changed.is_empty() {
                break;
            }
            moved = true;
            let mut seen = vec![false; members.len()];
            let mut affected = Vec::new();
            while let Some(position) = changed.pop() {
                crate::semantics::counted_walk::CountedWalk::HistoryReadsSettleKinds.step();
                match seen.get_mut(position) {
                    Some(mark) if !*mark => *mark = true,
                    Some(_) | None => continue,
                }
                affected.push(position);
                changed.extend(readers.get(position).into_iter().flatten().copied());
            }
            affected.sort_by_key(|position| rank.get(*position).copied().unwrap_or(0));
            for position in &affected {
                if let Some(node) = members.get(*position) {
                    self.memo.sigs.remove(node);
                }
            }
            if !self.run_members(&members, &affected, need)? {
                return self.abandon_kinds(&members, &reads, saved);
            }
            let mut next: Vec<usize> = affected
                .iter()
                .filter_map(|position| watching.get(position))
                .flatten()
                .copied()
                .collect();
            next.sort_unstable();
            next.dedup();
            candidates = next;
        }
        if moved {
            self.restore(&saved);
            for node in &members {
                self.memo.sigs.remove(node);
            }
            if !self.run_members(&members, &order, need)? {
                return self.abandon_kinds(&members, &reads, saved);
            }
        }
        Ok(true)
    }

    fn run_members(
        &mut self,
        members: &[NodeId],
        positions: &[usize],
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let current = self.current;
        let mut missing = Vec::new();
        for position in positions {
            let node = *members
                .get(*position)
                .ok_or(Internal::missed("kind group"))?;
            self.current = Some(Query::Sig(node));
            let done = self.run_sig(node, &mut missing)?;
            self.current = current;
            if !done {
                need.extend(missing);
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn restore(&mut self, saved: &Saved) {
        self.deferred.truncate(saved.deferred);
        self.memo.bare_matches.truncate(saved.matches);
        self.memo.takes_family.clone_from(&saved.families);
        self.memo.template_takers.clone_from(&saved.takers);
    }

    fn abandon_kinds(
        &mut self,
        members: &[NodeId],
        reads: &[(NodeId, BindingId, Option<NodeId>)],
        saved: Saved,
    ) -> Result<bool, Internal> {
        self.restore(&saved);
        for node in members {
            self.memo.sigs.remove(node);
        }
        for (read, ..) in reads {
            self.memo.history_kinds.remove(read);
        }
        Ok(false)
    }

    pub(super) fn settle_provisional_kinds(&mut self) -> Result<(), Internal> {
        let mut reads = std::mem::take(&mut self.memo.provisional_reads);
        reads.sort_unstable();
        reads.dedup();
        let provisional: Vec<(NodeId, Option<Sig>)> = reads
            .iter()
            .map(|read| (*read, self.memo.sigs.get(read).copied()))
            .collect();
        let mut changed = Vec::new();
        let mut moved: FastSet<u32> = FastSet::default();
        for (read, provisional) in provisional {
            let Op::History { cell, .. } = self.node(read)?.op else {
                continue;
            };
            if self.own_read(read, cell)? {
                continue;
            }
            let Some(provisional) = provisional else {
                continue;
            };
            let Some(group) = self.driven(|engine, need| engine.kind_component(read, need))? else {
                continue;
            };
            let members = match group {
                Some(group) => {
                    if !self.memo.history_kinds.contains_key(&read) {
                        self.driven(|engine, need| {
                            Ok(engine.settle_kinds(group, need)?.then_some(()))
                        })?;
                    }
                    let settled = self.known_sig(read)?;
                    if (settled.kind, settled.family) == (provisional.kind, provisional.family)
                        || !moved.insert(group)
                    {
                        continue;
                    }
                    self.memo
                        .kind_groups
                        .get(group as usize)
                        .cloned()
                        .ok_or(Internal::missed("kind group"))?
                }
                None => {
                    let Some(binding) = self.history_bindings(read)?.first().copied() else {
                        continue;
                    };
                    let init = self.binding(binding)?.init;
                    let Some(init) = self.driven(|engine, need| Ok(engine.sig(init, need)))? else {
                        continue;
                    };
                    let sig = self.read_sig(binding, init)?;
                    self.memo.history_kinds.insert(read, Some(sig));
                    self.memo.sigs.remove(&read);
                    self.forget_drawn(&[read]);
                    self.demand_sig(read)?;
                    vec![read]
                }
            };
            let settled = self.known_sig(read)?;
            if settled.kind != provisional.kind || settled.family != provisional.family {
                changed.extend(members);
            }
        }
        if !changed.is_empty() {
            self.invalidate_readers(&changed)?;
        }
        Ok(())
    }

    fn driven<T>(
        &mut self,
        mut step: impl FnMut(&mut Self, &mut Vec<Query>) -> Result<Option<T>, Internal>,
    ) -> Result<Option<T>, Internal> {
        let mut previous: Option<Vec<Query>> = None;
        loop {
            let mut need = Vec::new();
            if let Some(found) = step(self, &mut need)? {
                return Ok(Some(found));
            }
            if need.is_empty() || previous.as_ref() == Some(&need) {
                return Ok(None);
            }
            for query in &need {
                self.demand(*query)?;
            }
            previous = Some(need);
        }
    }

    fn invalidate_readers(&mut self, members: &[NodeId]) -> Result<(), Internal> {
        let nodes: Vec<NodeId> = self.memo.sigs.keys().copied().collect();
        let mut readers: FastMap<NodeId, Vec<NodeId>> = FastMap::default();
        for node in nodes {
            let mut ignored = Vec::new();
            for target in self.kind_edges(node, false, &mut ignored)? {
                readers.entry(target).or_default().push(node);
            }
        }
        let mut seen: FastSet<NodeId> = members.iter().copied().collect();
        let mut stack = members.to_vec();
        let mut removed = Vec::new();
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::HistoryReadsInvalidateReaders.step();
            for reader in readers.get(&node).into_iter().flatten() {
                if seen.insert(*reader) {
                    self.memo.sigs.remove(reader);
                    self.memo.history_kinds.remove(reader);
                    removed.push(*reader);
                    stack.push(*reader);
                }
            }
        }
        self.forget_drawn(&removed);
        for node in removed {
            self.demand_sig(node)?;
        }
        Ok(())
    }

    fn forget_drawn(&mut self, nodes: &[NodeId]) {
        let forgotten: FastSet<NodeId> = nodes.iter().copied().collect();
        let drawn = |deferred: &crate::semantics::analysis::deferred::Deferred| {
            deferred
                .signing
                .is_some_and(|node| forgotten.contains(&node))
        };
        if let Some(stop) = self.stop.as_mut() {
            let before = self.deferred.get(..stop.deferred).unwrap_or_default();
            let dropped = before.iter().filter(|deferred| drawn(deferred)).count();
            stop.deferred = stop.deferred.saturating_sub(dropped);
        }
        self.deferred.retain(|deferred| !drawn(deferred));
    }

    fn unsettled_kinds(
        &mut self,
        reads: &[(NodeId, BindingId, Option<NodeId>)],
    ) -> Result<bool, Internal> {
        for (read, ..) in reads {
            self.memo.history_kinds.insert(*read, None);
        }
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn history_bindings(
        &self,
        node: NodeId,
    ) -> Result<Vec<BindingId>, Internal> {
        let Op::History { cell, .. } = self.node(node)?.op else {
            return Ok(Vec::new());
        };
        let bound = match cell {
            HistoryCell::Input(input) => self.memo.inputs.get(&input),
            HistoryCell::Let(id) => self
                .memo
                .lets
                .get(&(self.global_of_let(id), LetRef::Top(id))),
            HistoryCell::State(state) => self.memo.states.get(&state),
        };
        Ok(match bound {
            Some(BoundVal::Scalar(Bound::Cell(binding))) => vec![*binding],
            Some(BoundVal::Array(array)) => array
                .1
                .iter()
                .filter_map(|item| match item {
                    Bound::Cell(binding) => Some(*binding),
                    Bound::Value(_) => None,
                })
                .collect(),
            Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => Vec::new(),
        })
    }
}

struct Saved {
    deferred: usize,
    matches: usize,
    families: FastMap<NodeId, symphonyscript_core::FamilyId>,
    takers: FastMap<crate::semantics::analysis::ids::ScopeId, Vec<NodeId>>,
}
