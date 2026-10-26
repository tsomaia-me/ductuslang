use std::collections::{HashMap, VecDeque};

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{InstanceId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::collection::strongly_connected;
use crate::semantics::reporting::grouped;

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
