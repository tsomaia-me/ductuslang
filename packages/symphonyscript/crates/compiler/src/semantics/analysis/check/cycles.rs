use std::collections::{HashMap, VecDeque};

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::{InstanceId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::query::Query;
use crate::semantics::collection::strongly_connected;
use crate::semantics::ir::Loc;
use crate::semantics::reporting::grouped;

type Identity = ((Loc, String), bool);

impl Engine<'_> {
    pub(super) fn wire_cycles(&mut self) -> Result<(), Internal> {
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); self.instances.len()];
        let mut wire_edges: Vec<(WireId, usize, Vec<usize>, bool)> = Vec::new();
        for index in 0..self.wires.len() {
            let wire = WireId(u32::try_from(index).unwrap_or(u32::MAX));
            let (kind, source) = {
                let record = self.wire(wire)?;
                (record.kind, record.source)
            };
            let cyclic = self.wire_decl(kind)?.cyclic;
            let destination = self.checked_destination(wire)?;
            let mut candidates = destination.candidates();
            candidates.sort();
            candidates.dedup();
            let mut targets = Vec::new();
            for candidate in candidates {
                self.memo.cycle_work += 1;
                if let ItemRef::Instance(target) = candidate {
                    edges
                        .get_mut(source.index())
                        .ok_or(Internal::missed("instance"))?
                        .push(target.index());
                    targets.push(target.index());
                }
            }
            wire_edges.push((wire, source.index(), targets, cyclic));
        }
        let components = strongly_connected(&edges)?;
        let mut component_of: Vec<usize> = vec![0; edges.len()];
        for (position, component) in components.iter().enumerate() {
            for member in component {
                self.memo.cycle_work += 1;
                *component_of
                    .get_mut(*member)
                    .ok_or(Internal::missed("instance"))? = position;
            }
        }
        let component = |node: usize| {
            component_of
                .get(node)
                .copied()
                .ok_or(Internal::missed("instance"))
        };
        let mut wire_counts: Vec<u64> = vec![0; components.len()];
        let mut first: Vec<Option<(WireId, usize, Vec<usize>)>> = vec![None; components.len()];
        for (wire, source, targets, cyclic) in wire_edges {
            let home = component(source)?;
            let inside: Vec<usize> = targets
                .into_iter()
                .map(|target| Ok((target, component(target)?)))
                .collect::<Result<Vec<_>, Internal>>()?
                .into_iter()
                .filter_map(|(target, of)| (of == home).then_some(target))
                .collect();
            if inside.is_empty() {
                continue;
            }
            *wire_counts
                .get_mut(home)
                .ok_or(Internal::missed("component"))? += 1;
            let slot = first.get_mut(home).ok_or(Internal::missed("component"))?;
            if !cyclic && slot.is_none() {
                *slot = Some((wire, source, inside));
            }
        }
        for (home, found) in first.into_iter().enumerate() {
            let Some((wire, source, targets)) = found else {
                continue;
            };
            let path = self.cycle_path(&edges, &component_of, home, source, &targets)?;
            let names: Vec<String> = path
                .iter()
                .map(|member| {
                    self.instance_label(InstanceId(u32::try_from(*member).unwrap_or(u32::MAX)))
                })
                .collect::<Result<_, _>>()?;
            let count = wire_counts
                .get(home)
                .copied()
                .ok_or(Internal::missed("component"))?;
            let (kind, loc, uses) = {
                let record = self.wire(wire)?;
                (record.kind, record.loc, record.uses.clone())
            };
            let kind_name = self.wire_decl(kind)?.name.clone();
            let held = if count == 1 {
                String::from("1 wire")
            } else {
                format!("{} wires", grouped(count))
            };
            self.defer(
                Code::WireCycle,
                loc,
                Vec::new(),
                uses,
                format!(
                    "this `{kind_name}` wire closes the cycle of wires {}, and its group of cycles holds {held}; `{kind_name}` wires form no cycles",
                    names.join(" → ")
                ),
            );
        }
        Ok(())
    }

    pub(crate) fn draw_declaration_cycles(&mut self) -> Result<(), Internal> {
        let cycles = std::mem::take(&mut self.found_cycles);
        let reads = std::mem::take(self.declaration_reads.get_mut());
        if cycles.is_empty() {
            return Ok(());
        }
        let mut found: Vec<(Loc, String)> = cycles.iter().flatten().cloned().collect();
        found.sort();
        found.dedup();
        let mut identities: FastMap<Query, Option<Identity>> = FastMap::default();
        let mut read_edges = Vec::new();
        let mut reads: Vec<(Query, Query)> = reads.into_iter().collect();
        reads.sort_by_key(|(from, to)| (format!("{from:?}"), format!("{to:?}")));
        for (from, to) in reads {
            self.memo.cycle_work += 1;
            let (Some((from, from_function)), Some((to, to_function))) = (
                self.identity(&mut identities, from)?,
                self.identity(&mut identities, to)?,
            ) else {
                continue;
            };
            if from != to {
                read_edges.push(((from, from_function), (to, to_function)));
            }
        }
        let mut members = found.clone();
        let mut functions: Vec<(Loc, String)> = Vec::new();
        for ((from, from_function), (to, to_function)) in &read_edges {
            for (member, function) in [(from, from_function), (to, to_function)] {
                members.push(member.clone());
                if *function {
                    functions.push(member.clone());
                }
            }
        }
        members.sort();
        members.dedup();
        functions.sort();
        functions.dedup();
        let position = |member: &(Loc, String)| {
            members
                .binary_search(member)
                .map_err(|_| Internal::missed("cycle declaration"))
        };
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); members.len()];
        for cycle in &cycles {
            for (step, from) in cycle.iter().enumerate() {
                let to = cycle
                    .get((step + 1) % cycle.len())
                    .ok_or(Internal::missed("cycle declaration"))?;
                edges
                    .get_mut(position(from)?)
                    .ok_or(Internal::missed("cycle declaration"))?
                    .push(position(to)?);
            }
        }
        let mut reads_only: Vec<Vec<usize>> = vec![Vec::new(); members.len()];
        for ((from, _), (to, _)) in &read_edges {
            reads_only
                .get_mut(position(from)?)
                .ok_or(Internal::missed("cycle declaration"))?
                .push(position(to)?);
            edges
                .get_mut(position(from)?)
                .ok_or(Internal::missed("cycle declaration"))?
                .push(position(to)?);
        }
        for targets in edges.iter_mut().chain(reads_only.iter_mut()) {
            targets.sort_unstable();
            targets.dedup();
        }
        let function: Vec<bool> = members
            .iter()
            .map(|member| functions.binary_search(member).is_ok())
            .collect();
        let components = strongly_connected(&edges)?;
        let mut component_of: Vec<usize> = vec![0; edges.len()];
        for (home, component) in components.iter().enumerate() {
            for member in component {
                *component_of
                    .get_mut(*member)
                    .ok_or(Internal::missed("cycle declaration"))? = home;
            }
        }
        for (home, component) in components.iter().enumerate() {
            let holds_found = component.iter().any(|member| {
                members
                    .get(*member)
                    .is_some_and(|member| found.binary_search(member).is_ok())
            });
            if !holds_found {
                continue;
            }
            let &first = component
                .iter()
                .min()
                .ok_or(Internal::missed("cycle declaration"))?;
            let read_path =
                self.declaration_path(&reads_only, (&component_of, home), first, &function)?;
            let path = match read_path {
                Some(path) => Some(path),
                None => self.declaration_path(&edges, (&component_of, home), first, &function)?,
            };
            let Some(path) = path else {
                continue;
            };
            let mut names = Vec::new();
            let mut locs = Vec::new();
            for member in path {
                let (loc, name) = members
                    .get(member)
                    .ok_or(Internal::missed("cycle declaration"))?;
                names.push(name.clone());
                locs.push(*loc);
            }
            let &loc = locs.first().ok_or(Internal::missed("cycle declaration"))?;
            let related: Vec<(Loc, String)> = locs
                .get(1..locs.len() - 1)
                .ok_or(Internal::missed("cycle declaration"))?
                .iter()
                .map(|other| (*other, "a declaration on the cycle".to_string()))
                .collect();
            let message = format!("these declarations reach themselves: {}", names.join(" → "));
            self.report
                .error_with(self.hir, Code::DeclarationCycle, loc, &related, message);
        }
        Ok(())
    }

    fn identity(
        &self,
        identities: &mut FastMap<Query, Option<Identity>>,
        query: Query,
    ) -> Result<Option<Identity>, Internal> {
        if let Some(known) = identities.get(&query) {
            return Ok(known.clone());
        }
        let found = self
            .declaration(query)?
            .map(|(name, loc)| ((loc, name), matches!(query, Query::Expr(..))));
        identities.insert(query, found.clone());
        Ok(found)
    }

    fn declaration_path(
        &mut self,
        edges: &[Vec<usize>],
        (component_of, home): (&[usize], usize),
        source: usize,
        function: &[bool],
    ) -> Result<Option<Vec<usize>>, Internal> {
        let declares = |node: usize| !function.get(node).copied().unwrap_or(false);
        let start = (source, declares(source));
        let mut previous: HashMap<(usize, bool), (usize, bool)> = HashMap::new();
        let mut queue = VecDeque::from([start]);
        let mut end = None;
        while let Some((node, seen)) = queue.pop_front() {
            self.memo.cycle_work += 1;
            for next in edges.get(node).ok_or(Internal::missed("instance"))? {
                self.memo.cycle_work += 1;
                if component_of.get(*next) != Some(&home) {
                    continue;
                }
                let state = (*next, seen || declares(*next));
                if *next == source && state.1 {
                    previous.insert((usize::MAX, true), (node, seen));
                    end = Some((node, seen));
                    break;
                }
                if *next == source || previous.contains_key(&state) || state == start {
                    continue;
                }
                previous.insert(state, (node, seen));
                queue.push_back(state);
            }
            if end.is_some() {
                break;
            }
        }
        let Some(mut current) = end else {
            return Ok(None);
        };
        let mut path = vec![source];
        while current != start {
            path.push(current.0);
            current = *previous
                .get(&current)
                .ok_or(Internal::missed("cycle step"))?;
        }
        path.push(source);
        path.reverse();
        Ok(Some(path))
    }

    fn cycle_path(
        &mut self,
        edges: &[Vec<usize>],
        component_of: &[usize],
        home: usize,
        source: usize,
        targets: &[usize],
    ) -> Result<Vec<usize>, Internal> {
        let mut previous: HashMap<usize, usize> = HashMap::new();
        let mut queue = VecDeque::new();
        for target in targets {
            if previous.insert(*target, source).is_none() {
                queue.push_back(*target);
            }
        }
        while let Some(node) = queue.pop_front() {
            self.memo.cycle_work += 1;
            if node == source {
                break;
            }
            for next in edges.get(node).ok_or(Internal::missed("instance"))? {
                self.memo.cycle_work += 1;
                if component_of.get(*next) != Some(&home) || previous.contains_key(next) {
                    continue;
                }
                previous.insert(*next, node);
                queue.push_back(*next);
            }
        }
        let mut path = vec![source];
        let mut current = *previous
            .get(&source)
            .ok_or(Internal::missed("cycle step"))?;
        while current != source {
            path.push(current);
            current = *previous
                .get(&current)
                .ok_or(Internal::missed("cycle step"))?;
        }
        path.push(source);
        path.reverse();
        Ok(path)
    }

    pub(crate) fn instance_label(&self, instance: InstanceId) -> Result<String, Internal> {
        let record = self.instance(instance)?;
        Ok(match &record.name {
            Some(name) => name.clone(),
            None => format!("a `{}`", self.kind_name(record.kind)?),
        })
    }
}
