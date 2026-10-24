use std::collections::{HashMap, HashSet};

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::entry::Entry;
use crate::semantics::analysis::ids::InstanceId;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::ir::{Loc, PlacementId};

impl Engine<'_> {
    pub(super) fn reachability(&mut self) -> Result<(), Internal> {
        let has_entry_kind = self.vocabulary.node_kinds().iter().any(|decl| decl.entry);
        if !has_entry_kind {
            return Ok(());
        }
        let mut entries: Vec<InstanceId> = Vec::new();
        for (index, record) in self.instances.iter().enumerate() {
            if self.node_decl(record.kind)?.entry {
                entries.push(InstanceId(u32::try_from(index).unwrap_or(u32::MAX)));
            }
        }
        if entries.is_empty() {
            if self.unknown_placement {
                return Ok(());
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
            return Ok(());
        }
        if self.unknown_wires || !self.unknown_destinations.is_empty() {
            return Ok(());
        }
        let mut reached: HashSet<InstanceId> = HashSet::new();
        let mut stack = entries;
        while let Some(instance) = stack.pop() {
            if !reached.insert(instance) {
                continue;
            }
            let record = self.instance(instance)?;
            for entry in &record.sequence {
                match entry {
                    Entry::Child(child) => stack.push(*child),
                    Entry::Bundle(members) => stack.extend(members.iter().copied()),
                    Entry::Wire(wire) => {
                        let destination = self.checked_destination(*wire)?;
                        for candidate in destination.candidates() {
                            if let ItemRef::Instance(target) = candidate {
                                stack.push(target);
                            }
                        }
                    }
                }
            }
        }
        let sites: HashMap<InstanceId, PlacementId> = self
            .top_instances
            .iter()
            .map(|(placement, instance)| (*instance, *placement))
            .collect();
        for instance in self.top_level.clone() {
            if reached.contains(&instance) {
                continue;
            }
            let record = self.instance(instance)?;
            let placement = self.hir.placement(record.placement)?;
            let loc = placement.name.as_ref().map_or(record.loc, |name| name.loc);
            let label = self.instance_label(instance)?;
            let top = sites.get(&instance).copied();
            let loc = match top {
                Some(pid) => self
                    .hir
                    .placement(pid)?
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
        Ok(())
    }
}
