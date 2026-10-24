use std::collections::{HashMap, VecDeque};

use crate::diagnostics::Code;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{InstanceId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::collection::strongly_connected;

impl Engine<'_> {
    pub(super) fn wire_cycles(&mut self) {
        let mut edges: Vec<Vec<usize>> = vec![Vec::new(); self.instances.len()];
        let mut wire_edges: Vec<(WireId, usize, usize, bool)> = Vec::new();
        for index in 0..self.wires.len() {
            let wire = WireId(u32::try_from(index).unwrap_or(u32::MAX));
            let (kind, source) = {
                let record = self.wire(wire);
                (record.kind, record.source)
            };
            let cyclic = self
                .vocabulary
                .wire_kind(kind)
                .is_some_and(|decl| decl.cyclic);
            let destination = self
                .memo
                .destinations
                .get(&wire)
                .cloned()
                .unwrap_or(Destination::Error);
            let mut candidates = destination.candidates();
            candidates.sort();
            candidates.dedup();
            for candidate in candidates {
                if let ItemRef::Instance(target) = candidate {
                    if let Some(targets) = edges.get_mut(source.index()) {
                        targets.push(target.index());
                    }
                    wire_edges.push((wire, source.index(), target.index(), cyclic));
                }
            }
        }
        let components = strongly_connected(&edges);
        let mut component_of: HashMap<usize, usize> = HashMap::new();
        for (position, component) in components.iter().enumerate() {
            for member in component {
                component_of.insert(*member, position);
            }
        }
        let mut reported = std::collections::HashSet::new();
        for (wire, source, target, cyclic) in wire_edges {
            let component = component_of.get(&source);
            if cyclic || component != component_of.get(&target) {
                continue;
            }
            if !reported.insert(component.copied()) {
                continue;
            }
            let path = self.cycle_path(&edges, target, source);
            let names: Vec<String> = path
                .iter()
                .map(|member| {
                    self.instance_label(InstanceId(u32::try_from(*member).unwrap_or(u32::MAX)))
                })
                .collect();
            let kind = self.wire(wire).kind;
            let kind_name = self
                .vocabulary
                .wire_kind(kind)
                .map_or(String::new(), |decl| decl.name.clone());
            let loc = self.wire(wire).loc;
            let uses = self.wire(wire).uses.clone();
            self.report.error_with(
                self.hir,
                Code::WireCycle,
                loc,
                &uses.to_vec(),
                format!(
                    "this `{kind_name}` wire closes a cycle of wires, {}, and `{kind_name}` wires form no cycles",
                    names.join(" → ")
                ),
            );
        }
    }

    fn cycle_path(&self, edges: &[Vec<usize>], from: usize, to: usize) -> Vec<usize> {
        let mut previous: HashMap<usize, usize> = HashMap::new();
        let mut queue = VecDeque::from([from]);
        previous.insert(from, from);
        while let Some(node) = queue.pop_front() {
            if node == to {
                break;
            }
            for next in edges.get(node).into_iter().flatten() {
                if !previous.contains_key(next) {
                    previous.insert(*next, node);
                    queue.push_back(*next);
                }
            }
        }
        let mut path = vec![to];
        let mut current = to;
        while current != from {
            let Some(&back) = previous.get(&current) else {
                break;
            };
            current = back;
            path.push(current);
        }
        path.reverse();
        path.push(from);
        path.dedup();
        if path.first() != path.last() {
            path.push(from);
        }
        path
    }

    pub(crate) fn instance_label(&self, instance: InstanceId) -> String {
        let record = self.instance(instance);
        match &record.name {
            Some(name) => name.clone(),
            None => format!("a `{}`", self.kind_name(record.kind)),
        }
    }
}
