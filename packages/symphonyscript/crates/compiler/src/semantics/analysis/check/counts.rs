use std::collections::BTreeMap;

use symphonyscript_core::{Count, NodeKind, WireKind};

use crate::diagnostics::Code;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::entry::Entry;
use crate::semantics::analysis::ids::{InstanceId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::ir::Loc;

impl Engine<'_> {
    pub(crate) fn check_program(&mut self) {
        self.top_level_counts();
        for index in 0..self.instances.len() {
            let instance = InstanceId(u32::try_from(index).unwrap_or(u32::MAX));
            self.children_counts(instance);
        }
        self.wire_counts();
        self.wire_cycles();
        self.reachability();
    }

    fn item_error(&mut self, code: Code, item: ItemRef, loc: Loc, message: String) {
        let uses = match item {
            ItemRef::Instance(id) => self.instance(id).uses.clone(),
            ItemRef::Wire(id) => self.wire(id).uses.clone(),
            ItemRef::Use(id) => self.pattern_use(id).uses.clone(),
        };
        self.report
            .error_with(self.hir, code, loc, &uses.to_vec(), message);
    }

    pub(crate) fn kind_name(&self, kind: NodeKind) -> String {
        self.vocabulary
            .node_kind(kind)
            .map_or(String::new(), |decl| decl.name.clone())
    }

    fn wire_kind_name(&self, kind: WireKind) -> String {
        self.vocabulary
            .wire_kind(kind)
            .map_or(String::new(), |decl| decl.name.clone())
    }

    fn top_level_counts(&mut self) {
        let mut by_kind: BTreeMap<NodeKind, Vec<InstanceId>> = BTreeMap::new();
        for instance in self.top_level.clone() {
            let kind = self.instance(instance).kind;
            let allowed = self
                .vocabulary
                .node_kind(kind)
                .and_then(|decl| decl.top_level);
            if allowed.is_none() {
                let loc = self.instance(instance).loc;
                let name = self.kind_name(kind);
                self.item_error(
                    Code::NotTopLevel,
                    ItemRef::Instance(instance),
                    loc,
                    format!("a `{name}` does not stand at top level"),
                );
                continue;
            }
            by_kind.entry(kind).or_default().push(instance);
        }
        for index in 0..self.vocabulary.node_kinds().len() {
            let Some(kind) = self.vocabulary.node_kind_at(index) else {
                continue;
            };
            let Some(count) = self
                .vocabulary
                .node_kind(kind)
                .and_then(|decl| decl.top_level)
            else {
                continue;
            };
            let found = by_kind.get(&kind).cloned().unwrap_or_default();
            let name = self.kind_name(kind);
            if let Some(max) = count.max()
                && let Some(extra) = found.get(max as usize)
            {
                let loc = self.instance(*extra).loc;
                self.item_error(
                    Code::CountExceeded,
                    ItemRef::Instance(*extra),
                    loc,
                    format!(
                        "the program holds more top-level `{name}` instances than {}",
                        describe(count)
                    ),
                );
            }
            if found.len() < count.min() as usize {
                let loc = Loc {
                    file: u32::try_from(self.hir.prelude.files).unwrap_or(0),
                    start: 0,
                    end: 0,
                };
                self.report.error(
                    self.hir,
                    Code::CountBelowMinimum,
                    loc,
                    format!(
                        "the program holds {} top-level `{name}`, and it needs {}",
                        found.len(),
                        describe(count)
                    ),
                );
            }
        }
    }

    pub(crate) fn children_of(&self, instance: InstanceId) -> Vec<InstanceId> {
        let mut children = Vec::new();
        for entry in &self.instance(instance).sequence {
            match entry {
                Entry::Child(child) => children.push(*child),
                Entry::Bundle(members) => children.extend(members.iter().copied()),
                Entry::Wire(_) => {}
            }
        }
        children
    }

    fn children_counts(&mut self, parent: InstanceId) {
        let kind = self.instance(parent).kind;
        let Some(decl) = self.vocabulary.node_kind(kind).cloned() else {
            return;
        };
        let mut by_kind: BTreeMap<NodeKind, Vec<InstanceId>> = BTreeMap::new();
        for child in self.children_of(parent) {
            let child_kind = self.instance(child).kind;
            if !decl
                .children
                .iter()
                .any(|(accepted, _)| *accepted == child_kind)
            {
                let loc = self.instance(child).loc;
                let message = if decl.children.is_empty() {
                    format!(
                        "`{}` accepts no children, and its body holds only wires",
                        decl.name
                    )
                } else {
                    format!(
                        "`{}` does not accept a `{}` child",
                        decl.name,
                        self.kind_name(child_kind)
                    )
                };
                self.item_error(
                    Code::ChildNotAccepted,
                    ItemRef::Instance(child),
                    loc,
                    message,
                );
                continue;
            }
            by_kind.entry(child_kind).or_default().push(child);
        }
        for (child_kind, count) in &decl.children {
            let found = by_kind.get(child_kind).cloned().unwrap_or_default();
            let name = self.kind_name(*child_kind);
            if let Some(max) = count.max()
                && let Some(extra) = found.get(max as usize)
            {
                let loc = self.instance(*extra).loc;
                self.item_error(
                    Code::CountExceeded,
                    ItemRef::Instance(*extra),
                    loc,
                    format!(
                        "`{}` holds {} `{name}` children",
                        decl.name,
                        describe(*count)
                    ),
                );
            }
            if found.len() < count.min() as usize {
                let loc = self.instance(parent).loc;
                self.item_error(
                    Code::CountBelowMinimum,
                    ItemRef::Instance(parent),
                    loc,
                    format!(
                        "this `{}` holds {} `{name}` children, and it needs {}",
                        decl.name,
                        found.len(),
                        describe(*count)
                    ),
                );
            }
        }
    }

    fn wire_counts(&mut self) {
        let mut outgoing: BTreeMap<(InstanceId, WireKind), Vec<WireId>> = BTreeMap::new();
        let mut incoming_fixed: BTreeMap<(InstanceId, WireKind), Vec<WireId>> = BTreeMap::new();
        let mut incoming_any: BTreeMap<(InstanceId, WireKind), Vec<WireId>> = BTreeMap::new();
        for index in 0..self.wires.len() {
            let wire = WireId(u32::try_from(index).unwrap_or(u32::MAX));
            let (kind, source, loc) = {
                let record = self.wire(wire);
                (record.kind, record.source, record.loc)
            };
            outgoing.entry((source, kind)).or_default().push(wire);
            let destination = self
                .memo
                .destinations
                .get(&wire)
                .cloned()
                .unwrap_or(Destination::Error);
            let fixed = matches!(destination, Destination::Fixed(_));
            let mut candidates = destination.candidates();
            candidates.sort();
            candidates.dedup();
            let Some(decl) = self.vocabulary.wire_kind(kind).cloned() else {
                continue;
            };
            let source_kind = self.instance(source).kind;
            for candidate in candidates {
                let ItemRef::Instance(target) = candidate else {
                    self.item_error(
                        Code::DestinationType,
                        ItemRef::Wire(wire),
                        loc,
                        "a destination names an instance".to_string(),
                    );
                    continue;
                };
                let target_kind = self.instance(target).kind;
                if !decl.connects.contains(&(source_kind, target_kind)) {
                    self.item_error(
                        Code::WireEndpoint,
                        ItemRef::Wire(wire),
                        loc,
                        format!(
                            "a `{}` wire does not connect a `{}` to a `{}`",
                            decl.name,
                            self.kind_name(source_kind),
                            self.kind_name(target_kind)
                        ),
                    );
                }
                incoming_any.entry((target, kind)).or_default().push(wire);
                if fixed {
                    incoming_fixed.entry((target, kind)).or_default().push(wire);
                }
            }
        }
        for index in 0..self.instances.len() {
            let instance = InstanceId(u32::try_from(index).unwrap_or(u32::MAX));
            let kind = self.instance(instance).kind;
            let Some(decl) = self.vocabulary.node_kind(kind).cloned() else {
                continue;
            };
            for (wire_kind, count) in &decl.outgoing {
                let found = outgoing
                    .get(&(instance, *wire_kind))
                    .cloned()
                    .unwrap_or_default();
                self.wire_bounds(instance, *wire_kind, *count, &found, &found, "outgoing");
            }
            for (wire_kind, count) in &decl.incoming {
                let any = incoming_any
                    .get(&(instance, *wire_kind))
                    .cloned()
                    .unwrap_or_default();
                let fixed = incoming_fixed
                    .get(&(instance, *wire_kind))
                    .cloned()
                    .unwrap_or_default();
                self.wire_bounds(instance, *wire_kind, *count, &any, &fixed, "incoming");
            }
        }
    }

    fn wire_bounds(
        &mut self,
        instance: InstanceId,
        wire_kind: WireKind,
        count: Count,
        toward_max: &[WireId],
        toward_min: &[WireId],
        direction: &str,
    ) {
        let name = self.wire_kind_name(wire_kind);
        let kind = self.kind_name(self.instance(instance).kind);
        if let Some(max) = count.max()
            && toward_max.len() > max as usize
        {
            let wire = toward_max.get(max as usize).copied().unwrap_or(WireId(0));
            let loc = self.wire(wire).loc;
            self.item_error(
                Code::CountExceeded,
                ItemRef::Wire(wire),
                loc,
                format!(
                    "a `{kind}` takes {} {direction} `{name}` wires",
                    describe(count)
                ),
            );
        }
        if toward_min.len() < count.min() as usize {
            let loc = self.instance(instance).loc;
            self.item_error(
                Code::CountBelowMinimum,
                ItemRef::Instance(instance),
                loc,
                format!(
                    "this `{kind}` has {} {direction} `{name}` wires, and it needs {}",
                    toward_min.len(),
                    describe(count)
                ),
            );
        }
    }
}

fn describe(count: Count) -> String {
    match count {
        Count::Exactly(n) => format!("exactly {n}"),
        Count::AtLeast(n) => format!("at least {n}"),
        Count::AtMost(n) => format!("at most {n}"),
        Count::Between(n, m) => format!("between {n} and {m}"),
        Count::Any => "any number".to_string(),
    }
}
