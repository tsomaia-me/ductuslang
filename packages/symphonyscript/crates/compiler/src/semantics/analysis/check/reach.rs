use std::collections::HashSet;

use crate::diagnostics::Code;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::entry::Entry;
use crate::semantics::analysis::ids::InstanceId;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::ir::Loc;

impl Engine<'_> {
    pub(super) fn reachability(&mut self) {
        let has_entry_kind = self.vocabulary.node_kinds().iter().any(|decl| decl.entry);
        if !has_entry_kind {
            return;
        }
        let entries: Vec<InstanceId> = (0..self.instances.len())
            .map(|index| InstanceId(u32::try_from(index).unwrap_or(u32::MAX)))
            .filter(|instance| {
                self.vocabulary
                    .node_kind(self.instance(*instance).kind)
                    .is_some_and(|decl| decl.entry)
            })
            .collect();
        if entries.is_empty() {
            if self.unknown_placement {
                return;
            }
            let loc = Loc {
                file: u32::try_from(self.hir.prelude.files).unwrap_or(0),
                start: 0,
                end: 0,
            };
            self.report.error(
                self.hir,
                Code::NoEntryInstance,
                loc,
                "the program places no instance of an entry kind, so nothing reaches its instances",
            );
            return;
        }
        if self.unknown_wires || !self.unknown_destinations.is_empty() {
            return;
        }
        let mut reached: HashSet<InstanceId> = HashSet::new();
        let mut stack = entries;
        while let Some(instance) = stack.pop() {
            if !reached.insert(instance) {
                continue;
            }
            for entry in &self.instance(instance).sequence {
                match entry {
                    Entry::Child(child) => stack.push(*child),
                    Entry::Bundle(members) => stack.extend(members.iter().copied()),
                    Entry::Wire(wire) => {
                        let destination = self
                            .memo
                            .destinations
                            .get(wire)
                            .cloned()
                            .unwrap_or(Destination::Error);
                        for candidate in destination.candidates() {
                            if let ItemRef::Instance(target) = candidate {
                                stack.push(target);
                            }
                        }
                    }
                }
            }
        }
        for instance in self.top_level.clone() {
            if reached.contains(&instance) {
                continue;
            }
            let record = self.instance(instance);
            let placement = self.hir.placement(record.placement);
            let loc = placement.name.as_ref().map_or(record.loc, |name| name.loc);
            let label = self.instance_label(instance);
            let top = self
                .top_instances
                .iter()
                .find(|(_, id)| **id == instance)
                .map(|(pid, _)| *pid);
            let loc = match top {
                Some(pid) => self
                    .hir
                    .placement(pid)
                    .name
                    .as_ref()
                    .map_or(loc, |name| name.loc),
                None => loc,
            };
            self.report.error(
                self.hir,
                Code::UnreachableInstance,
                loc,
                format!("no entry-kind instance reaches `{label}` through children and wires"),
            );
        }
    }
}
