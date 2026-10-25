use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::op::Op;
use crate::semantics::ir::Loc;

enum Carried {
    Open,
    Enum(Scalar),
    Clash,
}

struct Flows {
    members: FastSet<BindingId>,
    values: FastMap<BindingId, Vec<BindingId>>,
    partners: FastMap<BindingId, Vec<BindingId>>,
}

impl Engine<'_> {
    pub(super) fn enum_bindings(&mut self) -> Result<(), Internal> {
        let readers_first = std::mem::take(&mut self.typing.awaiting_enum);
        self.typing.enum_expected.clear();
        let reads_first: Vec<BindingId> = readers_first.iter().rev().copied().collect();
        let flows = self.flows(&readers_first)?;
        let downstream = self.downstream_enums(&readers_first, &flows);
        let mut waiting = flows.members.clone();
        let mut blocked = FastSet::default();
        let mut by_uses = Vec::new();
        loop {
            let mut changed = false;
            for &binding in &reads_first {
                if !waiting.contains(&binding) || blocked.contains(&binding) {
                    continue;
                }
                let expected = downstream
                    .get(&binding)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let reads_blocked = flows
                    .values
                    .get(&binding)
                    .into_iter()
                    .flatten()
                    .any(|read| blocked.contains(read));
                if expected.len() > 1 || reads_blocked {
                    blocked.insert(binding);
                    changed = true;
                    continue;
                }
                match self.carried_enum(self.binding(binding)?.init)? {
                    Carried::Clash => {
                        waiting.remove(&binding);
                        self.typing.untyped.insert(binding);
                        changed = true;
                    }
                    Carried::Enum(own) => {
                        waiting.remove(&binding);
                        self.take_enum(binding, own, &flows)?;
                        changed = true;
                    }
                    Carried::Open => {
                        if let [own] = expected {
                            waiting.remove(&binding);
                            by_uses.push(binding);
                            self.take_enum(binding, *own, &flows)?;
                            changed = true;
                        }
                    }
                }
            }
            for &binding in &readers_first {
                if !waiting.contains(&binding) || blocked.contains(&binding) {
                    continue;
                }
                match self.enums_expected(binding).as_slice() {
                    [] => {}
                    [own] => {
                        waiting.remove(&binding);
                        by_uses.push(binding);
                        self.take_enum(binding, *own, &flows)?;
                        changed = true;
                    }
                    _ => {
                        blocked.insert(binding);
                        changed = true;
                    }
                }
            }
            let mut compared = Vec::new();
            for &binding in &readers_first {
                if !waiting.contains(&binding) || blocked.contains(&binding) {
                    continue;
                }
                let enums = distinct_enums(
                    flows
                        .partners
                        .get(&binding)
                        .into_iter()
                        .flatten()
                        .filter(|partner| !waiting.contains(partner))
                        .filter_map(|partner| self.typing.own.get(partner).copied()),
                );
                if !enums.is_empty() {
                    compared.push((binding, enums));
                }
            }
            for (binding, enums) in compared {
                changed = true;
                match enums.as_slice() {
                    [own] => {
                        waiting.remove(&binding);
                        self.take_enum(binding, *own, &flows)?;
                    }
                    _ => {
                        blocked.insert(binding);
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for binding in by_uses {
            if self.enums_expected(binding).len() > 1 {
                self.no_single_enum(binding, Vec::new())?;
            }
        }
        let remaining: Vec<BindingId> = readers_first
            .into_iter()
            .filter(|binding| waiting.contains(binding))
            .collect();
        self.flows_without_enum(&remaining, &flows)
    }

    fn downstream_enums(
        &self,
        readers_first: &[BindingId],
        flows: &Flows,
    ) -> FastMap<BindingId, Vec<Scalar>> {
        let mut readers: FastMap<BindingId, Vec<BindingId>> = FastMap::default();
        for (reader, reads) in &flows.values {
            for read in reads {
                if read != reader && flows.members.contains(read) {
                    readers.entry(*read).or_default().push(*reader);
                }
            }
        }
        let mut downstream: FastMap<BindingId, Vec<Scalar>> = FastMap::default();
        for &binding in readers_first {
            let from_readers: Vec<Scalar> = readers
                .get(&binding)
                .into_iter()
                .flatten()
                .flat_map(|reader| downstream.get(reader).cloned().unwrap_or_default())
                .collect();
            let enums =
                distinct_enums(self.enums_expected(binding).into_iter().chain(from_readers));
            downstream.insert(binding, enums);
        }
        downstream
    }

    fn flows(&self, pending: &[BindingId]) -> Result<Flows, Internal> {
        let members: FastSet<BindingId> = pending.iter().copied().collect();
        let mut values = FastMap::default();
        let mut partners = FastMap::default();
        for &binding in pending {
            values.insert(binding, self.value_reads(self.binding(binding)?.init)?);
            let mut compared = Vec::new();
            for partner in self.typing.equal_to.get(&binding).into_iter().flatten() {
                if let Op::Read(other) = self.node(self.through_uses(*partner)?)?.op
                    && other != binding
                    && members.contains(&other)
                {
                    compared.push(other);
                }
            }
            partners.insert(binding, compared);
        }
        for (binding, compared) in partners.clone() {
            for other in compared {
                let list: &mut Vec<BindingId> = partners.entry(other).or_default();
                if !list.contains(&binding) {
                    list.push(binding);
                }
            }
        }
        Ok(Flows {
            members,
            values,
            partners,
        })
    }

    fn enums_expected(&self, binding: BindingId) -> Vec<Scalar> {
        distinct_enums(
            self.typing
                .expectations
                .get(&binding)
                .into_iter()
                .flatten()
                .map(|(scalar, _)| *scalar),
        )
    }

    fn take_enum(
        &mut self,
        binding: BindingId,
        own: Scalar,
        flows: &Flows,
    ) -> Result<(), Internal> {
        self.typing.own.insert(binding, own);
        let init = self.binding(binding)?.init;
        self.expect(init, Some(own))?;
        let partners = self
            .typing
            .equal_to
            .get(&binding)
            .cloned()
            .unwrap_or_default();
        for partner in partners {
            let compared = match self.node(self.through_uses(partner)?)?.op {
                Op::Read(other) => flows.members.contains(&other),
                _ => false,
            };
            if !compared {
                self.expect(partner, Some(own))?;
            }
        }
        self.typing.enum_expected.clear();
        Ok(())
    }

    fn value_parts(op: &Op) -> Vec<NodeId> {
        match op {
            Op::Use(inner) => vec![*inner],
            Op::If(_, then, otherwise) => vec![*then, *otherwise],
            Op::Match {
                arms, otherwise, ..
            } => arms
                .iter()
                .map(|(_, value)| *value)
                .chain(*otherwise)
                .collect(),
            Op::Pick { items, .. } => items.clone(),
            Op::Element { base, .. } => vec![*base],
            Op::Array(elements) => elements.clone(),
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Read(_)
            | Op::Arg(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Convert(..)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::ToRead { .. } => Vec::new(),
        }
    }

    fn value_reads(&self, init: NodeId) -> Result<Vec<BindingId>, Internal> {
        let mut reads = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![init];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            let op = &self.node(node)?.op;
            if let Op::Read(binding) = op {
                reads.push(*binding);
            }
            stack.extend(Self::value_parts(op));
        }
        reads.sort();
        reads.dedup();
        Ok(reads)
    }

    fn carried_enum(&mut self, init: NodeId) -> Result<Carried, Internal> {
        let mut values: FastMap<NodeId, Option<Scalar>> = FastMap::default();
        let mut stack = vec![(init, false)];
        while let Some((node, ready)) = stack.pop() {
            if values.contains_key(&node) {
                continue;
            }
            let op = self.node(node)?.op.clone();
            let parts = Self::value_parts(&op);
            if !ready && !parts.is_empty() {
                stack.push((node, true));
                stack.extend(parts.into_iter().map(|part| (part, false)));
                continue;
            }
            let value = match op {
                Op::Read(binding) => self
                    .typing
                    .own
                    .get(&binding)
                    .copied()
                    .filter(|scalar| matches!(scalar, Scalar::Enum(_))),
                _ => {
                    let mut found = None;
                    for part in &parts {
                        let part_value = values
                            .get(part)
                            .copied()
                            .ok_or(Internal::missed("carried value"))?;
                        match (found, part_value) {
                            (_, None) => {}
                            (None, Some(value)) => found = Some(value),
                            (Some(first), Some(value)) if first == value => {}
                            (Some(_), Some(_)) => {
                                let message = match op {
                                    Op::If(..) => {
                                        "the branches of an `if` hold values of different types"
                                    }
                                    Op::Match { .. } => {
                                        "the arms of a `match` hold values of different types"
                                    }
                                    _ => "the elements of the array hold values of different types",
                                };
                                self.error_at_node(Code::TypeMismatch, node, message)?;
                                return Ok(Carried::Clash);
                            }
                        }
                    }
                    found
                }
            };
            values.insert(node, value);
        }
        Ok(match values.get(&init) {
            Some(Some(own)) => Carried::Enum(*own),
            Some(None) | None => Carried::Open,
        })
    }

    fn flows_without_enum(
        &mut self,
        remaining: &[BindingId],
        flows: &Flows,
    ) -> Result<(), Internal> {
        let mut read_by_untyped = FastSet::default();
        let untyped: Vec<BindingId> = self.typing.untyped.iter().copied().collect();
        for binding in &untyped {
            read_by_untyped.extend(self.reads_of(self.binding(*binding)?.init)?);
        }
        read_by_untyped.extend(self.typing.read_by_faults.iter().copied());
        if !self.cycle_spans.is_empty() {
            for node in &self.nodes {
                if let Op::Read(binding) = node.op
                    && self.cycle_spans.iter().any(|span| span.contains(node.loc))
                {
                    read_by_untyped.insert(binding);
                }
            }
        }
        let members: FastSet<BindingId> = remaining.iter().copied().collect();
        let mut locs: FastMap<BindingId, Loc> = FastMap::default();
        let mut writers = FastSet::default();
        let mut neighbours: FastMap<BindingId, Vec<BindingId>> = FastMap::default();
        let mut read_by_member = FastSet::default();
        for &binding in remaining {
            let record = self.binding(binding)?;
            let (loc, init) = (record.loc, record.init);
            locs.insert(binding, loc);
            if self.writes_variant(init)? {
                writers.insert(binding);
            }
            for read in self.reads_of(init)? {
                if read != binding && members.contains(&read) {
                    read_by_member.insert(read);
                }
            }
            let met = flows
                .values
                .get(&binding)
                .into_iter()
                .flatten()
                .chain(flows.partners.get(&binding).into_iter().flatten())
                .filter(|other| **other != binding && members.contains(other))
                .copied()
                .collect::<Vec<_>>();
            for other in met {
                neighbours.entry(binding).or_default().push(other);
                neighbours.entry(other).or_default().push(binding);
            }
        }
        for &binding in remaining {
            self.typing.untyped.insert(binding);
        }
        let mut seen = FastSet::default();
        for &start in remaining {
            if !seen.insert(start) {
                continue;
            }
            let mut flow = vec![start];
            let mut stack = vec![start];
            while let Some(binding) = stack.pop() {
                for other in neighbours.get(&binding).into_iter().flatten() {
                    if seen.insert(*other) {
                        flow.push(*other);
                        stack.push(*other);
                    }
                }
            }
            let live = flow.iter().any(|binding| {
                self.typing.reads.contains_key(binding)
                    || (!read_by_member.contains(binding) && !read_by_untyped.contains(binding))
            });
            let written = flow.iter().any(|binding| writers.contains(binding));
            let inherited = flow.iter().any(|binding| {
                flows
                    .values
                    .get(binding)
                    .into_iter()
                    .flatten()
                    .any(|read| untyped.contains(read))
            });
            if !live || (!written && inherited) {
                continue;
            }
            let mut located = Vec::with_capacity(flow.len());
            for binding in flow {
                let loc = locs
                    .get(&binding)
                    .copied()
                    .ok_or(Internal::missed("binding"))?;
                located.push((!writers.contains(&binding), loc, binding));
            }
            located.sort();
            let Some(((_, _, first), others)) = located.split_first() else {
                continue;
            };
            let mut secondary: Vec<Loc> = others.iter().map(|(_, loc, _)| *loc).collect();
            secondary.sort();
            self.no_single_enum(*first, secondary)?;
        }
        Ok(())
    }

    fn writes_variant(&self, init: NodeId) -> Result<bool, Internal> {
        let mut seen = FastSet::default();
        let mut stack = vec![init];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::Bare(_) => return Ok(true),
                Op::Read(_) => {}
                op => stack.extend(op.operands()),
            }
        }
        Ok(false)
    }

    fn no_single_enum(&mut self, binding: BindingId, secondary: Vec<Loc>) -> Result<(), Internal> {
        self.typing.untyped.insert(binding);
        self.typing.own.remove(&binding);
        let record = self.binding(binding)?;
        let (loc, init) = (record.loc, record.init);
        let scope = self.node(init)?.scope;
        if self.in_probe(scope)? {
            return Ok(());
        }
        self.error_with_chain(
            Code::TypeMismatch,
            loc,
            secondary,
            scope,
            "this value names a variant, and its uses expect no single enum",
        )
    }
}

fn distinct_enums(scalars: impl Iterator<Item = Scalar>) -> Vec<Scalar> {
    let mut enums = Vec::new();
    for scalar in scalars {
        if matches!(scalar, Scalar::Enum(_)) && !enums.contains(&scalar) {
            enums.push(scalar);
        }
    }
    enums
}
