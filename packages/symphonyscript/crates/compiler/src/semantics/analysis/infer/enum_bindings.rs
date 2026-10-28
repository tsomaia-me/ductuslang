use symphonyscript_core::{BinaryOp, EnumId, Scalar};

use super::position::Position;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::{BindingId, NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::ir::Loc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
enum Member {
    Cell(BindingId),
    Value(NodeId),
}

enum Carried {
    Open,
    Enum(Scalar),
    Clash,
}

struct Flows {
    members: FastSet<Member>,
    reads_first: Vec<Member>,
    values: FastMap<Member, Vec<Member>>,
    partners: FastMap<Member, Vec<Member>>,
    given: FastMap<NodeId, Vec<EnumId>>,
    inputs: FastSet<BindingId>,
    used: FastSet<NodeId>,
    lets: LetWriters,
}

struct LetWriters {
    aliases: FastMap<NodeId, Vec<Loc>>,
    values: Vec<(Loc, Loc)>,
}

impl Engine<'_> {
    pub(super) fn enum_bindings(&mut self, valid: &[Position]) -> Result<(), Internal> {
        let pending = std::mem::take(&mut self.typing.awaiting_enum);
        self.typing.enum_expected.clear();
        let flows = self.flows(&pending, valid)?;
        let reads_first = flows.reads_first.clone();
        let readers_first: Vec<Member> = reads_first.iter().rev().copied().collect();
        let downstream = self.downstream_enums(&readers_first, &flows);
        let mut pushed: FastMap<NodeId, Vec<Scalar>> = FastMap::default();
        let mut waiting = flows.members.clone();
        let mut blocked = FastSet::default();
        let mut by_uses = Vec::new();
        let order = FlowOrder::new(&reads_first, &readers_first, &flows);
        let mut dirty = Dirty::all(&order);
        let mut verified = false;
        loop {
            let mut changed = Vec::new();
            let mut cursor = 0;
            while let Some(position) = dirty.reads.range(cursor..).next().copied() {
                dirty.reads.remove(&position);
                cursor = position + 1;
                let member = order
                    .reads_first
                    .get(position)
                    .copied()
                    .ok_or(Internal::missed("flow member"))?;
                self.flow_step();
                if !waiting.contains(&member) || blocked.contains(&member) {
                    continue;
                }
                let expected = downstream
                    .get(&member)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let reads_blocked = flows
                    .values
                    .get(&member)
                    .into_iter()
                    .flatten()
                    .any(|read| blocked.contains(read));
                if expected.len() > 1 || reads_blocked {
                    blocked.insert(member);
                    dirty.touch(member, &order);
                    changed.push(member);
                    continue;
                }
                match self.carried_enum(self.member_node(member)?, &flows)? {
                    Carried::Clash => {
                        waiting.remove(&member);
                        self.mark_untyped(member)?;
                        dirty.touch(member, &order);
                        changed.push(member);
                    }
                    Carried::Enum(own) => {
                        waiting.remove(&member);
                        self.take_enum(member, own, &flows, &mut pushed)?;
                        dirty.touch(member, &order);
                        changed.push(member);
                    }
                    Carried::Open => {
                        if let [own] = expected {
                            waiting.remove(&member);
                            by_uses.push(member);
                            self.take_enum(member, *own, &flows, &mut pushed)?;
                            dirty.touch(member, &order);
                            changed.push(member);
                        }
                    }
                }
            }
            let mut cursor = 0;
            while let Some(position) = dirty.readers.range(cursor..).next().copied() {
                dirty.readers.remove(&position);
                cursor = position + 1;
                let member = order
                    .readers_first
                    .get(position)
                    .copied()
                    .ok_or(Internal::missed("flow member"))?;
                self.flow_step();
                if !waiting.contains(&member)
                    || blocked.contains(&member)
                    || is_input(member, &flows)
                {
                    continue;
                }
                match self.member_expected(member, &flows, &pushed).as_slice() {
                    [] => {}
                    [own] => {
                        waiting.remove(&member);
                        by_uses.push(member);
                        self.take_enum(member, *own, &flows, &mut pushed)?;
                        dirty.touch(member, &order);
                        changed.push(member);
                    }
                    _ => {
                        blocked.insert(member);
                        dirty.touch(member, &order);
                        changed.push(member);
                    }
                }
            }
            let mut compared = Vec::new();
            for position in std::mem::take(&mut dirty.partners) {
                let member = order
                    .readers_first
                    .get(position)
                    .copied()
                    .ok_or(Internal::missed("flow member"))?;
                self.flow_step();
                if !waiting.contains(&member)
                    || blocked.contains(&member)
                    || is_input(member, &flows)
                {
                    continue;
                }
                let enums = distinct_enums(
                    flows
                        .partners
                        .get(&member)
                        .into_iter()
                        .flatten()
                        .filter(|partner| !waiting.contains(partner))
                        .filter_map(|partner| self.member_enum(*partner)),
                );
                if !enums.is_empty() {
                    compared.push((member, enums));
                }
            }
            for (member, enums) in compared {
                dirty.touch(member, &order);
                changed.push(member);
                match enums.as_slice() {
                    [own] => {
                        waiting.remove(&member);
                        self.take_enum(member, *own, &flows, &mut pushed)?;
                    }
                    _ => {
                        blocked.insert(member);
                    }
                }
            }
            if !changed.is_empty() {
                verified = false;
                continue;
            }
            if verified {
                break;
            }
            verified = true;
            dirty = Dirty::all(&order);
        }
        for member in by_uses {
            if self.member_expected(member, &flows, &pushed).len() > 1 {
                let (_, scope) = self.member_site(member, &flows.lets)?;
                let loc = self.value_loc(member, &flows.lets)?;
                self.report_flow(member, loc, scope, Vec::new())?;
            }
        }
        let remaining: Vec<Member> = readers_first
            .into_iter()
            .filter(|member| waiting.contains(member))
            .collect();
        self.flows_without_enum(&remaining, &flows, &blocked)
    }

    fn flows(&self, pending: &[BindingId], valid: &[Position]) -> Result<Flows, Internal> {
        let mut members: FastSet<Member> = pending.iter().map(|cell| Member::Cell(*cell)).collect();
        let mut inputs = FastSet::default();
        for cell in pending {
            if matches!(self.binding(*cell)?.kind, BindingKind::Input { .. }) {
                inputs.insert(*cell);
            }
        }
        let mut carried = FastSet::default();
        for cell in pending {
            let mut stack = vec![self.binding(*cell)?.init];
            while let Some(node) = stack.pop() {
                crate::semantics::counted_walk::CountedWalk::EnumBindingsFlows.step();
                self.flow_step();
                if !carried.insert(node) {
                    continue;
                }
                let op = &self.node(node)?.op;
                if !matches!(op, Op::Use(_)) {
                    stack.extend(op.operands());
                }
            }
        }
        let mut used = FastSet::default();
        let mut ends = FastMap::default();
        for (index, node) in self.nodes.iter().enumerate() {
            crate::semantics::counted_walk::CountedWalk::EnumBindingsFlows.step();
            self.flow_step();
            let Op::Use(inner) = node.op else {
                continue;
            };
            if self.in_name_probe(node.scope)? {
                continue;
            }
            let target = self.use_end(inner, &mut ends)?;
            if !self.is_bare_value(target)? {
                continue;
            }
            members.insert(Member::Value(target));
            let id = NodeId(u32::try_from(index).map_err(|_| Internal::missed("node"))?);
            if !carried.contains(&id)
                && !self.cycle_spans.iter().any(|span| span.contains(node.loc))
            {
                used.insert(target);
            }
        }
        let read: FastSet<NodeId> = self.memo.groups.values().copied().collect();
        for group in &self.memo.group_list {
            crate::semantics::counted_walk::CountedWalk::EnumBindingsFlows.step();
            if read.contains(group) && self.is_bare_value(*group)? {
                members.insert(Member::Value(*group));
            }
        }
        let mut values = FastMap::default();
        for member in &members {
            self.flow_step();
            values.insert(*member, self.value_reads(*member, &members)?);
        }
        let mut partners: FastMap<Member, Vec<Member>> = FastMap::default();
        for node in &self.nodes {
            self.flow_step();
            let Op::Binary(BinaryOp::Is | BinaryOp::IsNot, lhs, rhs) = node.op else {
                continue;
            };
            let (Some(left), Some(right)) = (
                self.member_of(lhs, &members)?,
                self.member_of(rhs, &members)?,
            ) else {
                continue;
            };
            if left == right {
                continue;
            }
            for (from, to) in [(left, right), (right, left)] {
                let list = partners.entry(from).or_default();
                if !list.contains(&to) {
                    list.push(to);
                }
            }
        }
        let mut sorted: Vec<Member> = members.iter().copied().collect();
        sorted.sort();
        let mut reads_first = Vec::with_capacity(sorted.len());
        let mut done = FastSet::default();
        for start in sorted {
            let mut stack = vec![(start, false)];
            while let Some((member, expanded)) = stack.pop() {
                crate::semantics::counted_walk::CountedWalk::EnumBindingsFlows.step();
                self.flow_step();
                if done.contains(&member) {
                    continue;
                }
                if expanded {
                    done.insert(member);
                    reads_first.push(member);
                    continue;
                }
                stack.push((member, true));
                for read in values.get(&member).into_iter().flatten() {
                    if !done.contains(read) {
                        stack.push((*read, false));
                    }
                }
            }
        }
        Ok(Flows {
            members,
            reads_first,
            values,
            partners,
            given: self.compile_time_enums(valid)?,
            inputs,
            used,
            lets: self.let_writers()?,
        })
    }

    fn is_bare_value(&self, node: NodeId) -> Result<bool, Internal> {
        if matches!(self.node(node)?.op, Op::Recursive { .. } | Op::Arg(_)) {
            return Ok(false);
        }
        Ok(self
            .memo
            .sigs
            .get(&node)
            .is_some_and(|sig| !sig.reactive && sig.kind == Kind::Bare))
    }

    fn member_of(
        &self,
        node: NodeId,
        members: &FastSet<Member>,
    ) -> Result<Option<Member>, Internal> {
        let source = self.through_uses(node)?;
        let member = match self.node(source)?.op {
            Op::Read(binding) => Member::Cell(binding),
            _ => Member::Value(source),
        };
        Ok(members.contains(&member).then_some(member))
    }

    fn member_node(&self, member: Member) -> Result<NodeId, Internal> {
        Ok(match member {
            Member::Cell(binding) => self.binding(binding)?.init,
            Member::Value(node) => node,
        })
    }

    fn member_enum(&self, member: Member) -> Option<Scalar> {
        match member {
            Member::Cell(binding) => self.typing.own.get(&binding).copied(),
            Member::Value(node) => self.memo.bare_enums.get(&node).map(|id| Scalar::Enum(*id)),
        }
    }

    fn use_end(
        &self,
        node: NodeId,
        ends: &mut FastMap<NodeId, NodeId>,
    ) -> Result<NodeId, Internal> {
        let mut path = Vec::new();
        let mut current = node;
        let end = loop {
            if let Some(end) = ends.get(&current) {
                break *end;
            }
            self.memo.use_steps.set(self.memo.use_steps.get() + 1);
            match self.node(current)?.op {
                Op::Use(inner) => {
                    path.push(current);
                    current = inner;
                }
                _ => break current,
            }
        };
        for step in path {
            ends.insert(step, end);
        }
        Ok(end)
    }

    fn flow_step(&self) {
        self.memo.flow_work.set(self.memo.flow_work.get() + 1);
    }

    fn value_loc(&self, member: Member, lets: &LetWriters) -> Result<Loc, Internal> {
        self.flow_step();
        let loc = self.node(self.member_node(member)?)?.loc;
        Ok(value_of(lets, loc).unwrap_or(loc))
    }

    fn member_site(&self, member: Member, lets: &LetWriters) -> Result<(Loc, ScopeId), Internal> {
        self.flow_step();
        Ok(match member {
            Member::Cell(binding) => {
                let record = self.binding(binding)?;
                (record.loc, self.node(record.init)?.scope)
            }
            Member::Value(node) => {
                let record = self.node(node)?;
                (
                    writer_of(lets, record.loc).unwrap_or(record.loc),
                    record.scope,
                )
            }
        })
    }

    fn downstream_enums(
        &self,
        readers_first: &[Member],
        flows: &Flows,
    ) -> FastMap<Member, Vec<Scalar>> {
        let mut readers: FastMap<Member, Vec<Member>> = FastMap::default();
        for (reader, reads) in &flows.values {
            if is_input(*reader, flows) {
                continue;
            }
            for read in reads {
                readers.entry(*read).or_default().push(*reader);
            }
        }
        let mut downstream: FastMap<Member, Vec<Scalar>> = FastMap::default();
        for &member in readers_first {
            if is_input(member, flows) {
                continue;
            }
            let from_readers: Vec<Scalar> = readers
                .get(&member)
                .into_iter()
                .flatten()
                .flat_map(|reader| downstream.get(reader).cloned().unwrap_or_default())
                .collect();
            let own = self.member_expected(member, flows, &FastMap::default());
            downstream.insert(member, distinct_enums(own.into_iter().chain(from_readers)));
        }
        downstream
    }

    fn member_expected(
        &self,
        member: Member,
        flows: &Flows,
        pushed: &FastMap<NodeId, Vec<Scalar>>,
    ) -> Vec<Scalar> {
        match member {
            Member::Cell(binding) => distinct_enums(
                self.typing
                    .expectations
                    .get(&binding)
                    .into_iter()
                    .flatten()
                    .map(|(scalar, _)| *scalar),
            ),
            Member::Value(node) => distinct_enums(
                flows
                    .given
                    .get(&node)
                    .into_iter()
                    .flatten()
                    .map(|id| Scalar::Enum(*id))
                    .chain(pushed.get(&node).into_iter().flatten().copied()),
            ),
        }
    }

    fn take_enum(
        &mut self,
        member: Member,
        own: Scalar,
        flows: &Flows,
        pushed: &mut FastMap<NodeId, Vec<Scalar>>,
    ) -> Result<(), Internal> {
        for read in flows.values.get(&member).into_iter().flatten() {
            if let Member::Value(node) = read {
                let list = pushed.entry(*node).or_default();
                if !list.contains(&own) {
                    list.push(own);
                }
            }
        }
        let binding = match member {
            Member::Cell(binding) => binding,
            Member::Value(node) => {
                let Scalar::Enum(id) = own else {
                    return Err(Internal::missed("enum"));
                };
                for bare in self.value_bares(node)? {
                    self.memo.bare_enums.insert(bare, id);
                    let Op::Bare(name) = &self.node(bare)?.op else {
                        continue;
                    };
                    if self.variants.position(id, name)?.is_none()
                        && !self.in_name_probe(self.node(bare)?.scope)?
                    {
                        let message = format!(
                            "`{name}` is not a variant of `{}`",
                            self.enum_decl(id)?.name
                        );
                        self.error_at_node(Code::UnknownName, bare, message)?;
                    }
                }
                return Ok(());
            }
        };
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
            if self.member_of(partner, &flows.members)?.is_none() {
                self.expect(partner, Some(own))?;
            }
        }
        self.typing.enum_expected.clear();
        Ok(())
    }

    fn mark_untyped(&mut self, member: Member) -> Result<(), Internal> {
        match member {
            Member::Cell(binding) => {
                self.typing.untyped.insert(binding);
                self.typing.own.remove(&binding);
            }
            Member::Value(node) => {
                for bare in self.value_bares(node)? {
                    self.typing.untyped_values.insert(bare);
                }
            }
        }
        Ok(())
    }

    fn value_bares(&self, root: NodeId) -> Result<Vec<NodeId>, Internal> {
        let mut found = vec![root];
        let mut seen = FastSet::default();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::EnumBindingsValueBares.step();
            self.flow_step();
            if !seen.insert(node) {
                continue;
            }
            let op = &self.node(node)?.op;
            match op {
                Op::Bare(_) => found.push(node),
                Op::Use(_) | Op::Read(_) => {}
                _ => stack.extend(self.value_parts_of(node, op)),
            }
        }
        found.sort();
        found.dedup();
        Ok(found)
    }

    fn value_parts_of(&self, node: NodeId, op: &Op) -> Vec<NodeId> {
        let mut parts = Self::value_parts(op);
        parts.extend(self.memo.groups.get(&node).copied());
        if let Op::History {
            fallback: Some(fallback),
            ..
        } = op
            && self.memo.own_history.contains(&node)
        {
            parts.push(*fallback);
        }
        parts
    }

    fn other_history(&self, node: NodeId, op: &Op) -> bool {
        matches!(op, Op::History { .. }) && !self.memo.own_history.contains(&node)
    }

    fn value_parts(op: &Op) -> Vec<NodeId> {
        match op {
            Op::Use(inner) | Op::Chosen(inner) => vec![*inner],
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
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::History { .. }
            | Op::Native { .. } => Vec::new(),
        }
    }

    fn value_reads(
        &self,
        member: Member,
        members: &FastSet<Member>,
    ) -> Result<Vec<Member>, Internal> {
        let root = self.member_node(member)?;
        let mut reads = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::EnumBindingsValueReads.step();
            self.flow_step();
            if !seen.insert(node) {
                continue;
            }
            if let Some(group) = self.memo.groups.get(&node)
                && members.contains(&Member::Value(*group))
            {
                reads.push(Member::Value(*group));
            }
            let op = &self.node(node)?.op;
            if self.other_history(node, op) {
                for binding in self.history_bindings(node)? {
                    let read = Member::Cell(binding);
                    if read != member && members.contains(&read) {
                        reads.push(read);
                    }
                }
                continue;
            }
            let read = match op {
                Op::Read(binding) => Some(Member::Cell(*binding)),
                Op::Use(inner) => {
                    let target = self.through_uses(*inner)?;
                    members
                        .contains(&Member::Value(target))
                        .then_some(Member::Value(target))
                }
                _ if node != root && members.contains(&Member::Value(node)) => {
                    Some(Member::Value(node))
                }
                _ => None,
            };
            match read {
                Some(read) => {
                    if read != member && members.contains(&read) {
                        reads.push(read);
                    }
                }
                None => stack.extend(self.value_parts_of(node, op)),
            }
        }
        reads.sort();
        reads.dedup();
        Ok(reads)
    }

    fn carried_enum(&mut self, root: NodeId, flows: &Flows) -> Result<Carried, Internal> {
        let mut values: FastMap<NodeId, Option<Scalar>> = FastMap::default();
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::EnumBindingsCarriedEnum.step();
            if values.contains_key(&node) {
                continue;
            }
            let op = self.node(node)?.op.clone();
            let leaf = match &op {
                _ if self.other_history(node, &op) => Some(self.history_type(node)?),
                Op::Read(binding) => Some(self.typing.own.get(binding).copied()),
                Op::Use(inner) => {
                    let target = self.through_uses(*inner)?;
                    flows
                        .members
                        .contains(&Member::Value(target))
                        .then(|| self.member_enum(Member::Value(target)))
                }
                _ if node != root && flows.members.contains(&Member::Value(node)) => {
                    Some(self.member_enum(Member::Value(node)))
                }
                _ => None,
            };
            let leaf = leaf.map(|value| {
                value.or_else(|| {
                    self.memo
                        .groups
                        .get(&node)
                        .filter(|group| flows.members.contains(&Member::Value(**group)))
                        .and_then(|group| self.member_enum(Member::Value(*group)))
                })
            });
            if let Some(value) = leaf {
                values.insert(
                    node,
                    value.filter(|scalar| matches!(scalar, Scalar::Enum(_))),
                );
                continue;
            }
            let parts = self.value_parts_of(node, &op);
            if !ready && !parts.is_empty() {
                stack.push((node, true));
                stack.extend(parts.into_iter().map(|part| (part, false)));
                continue;
            }
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
                            Op::If(..) => "the branches of an `if` hold values of different types",
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
            values.insert(node, found);
        }
        Ok(match values.get(&root) {
            Some(Some(own)) => Carried::Enum(*own),
            Some(None) | None => Carried::Open,
        })
    }

    fn flows_without_enum(
        &mut self,
        remaining: &[Member],
        flows: &Flows,
        blocked: &FastSet<Member>,
    ) -> Result<(), Internal> {
        let mut read_by_untyped = FastSet::default();
        let untyped: Vec<BindingId> = self.typing.untyped.iter().copied().collect();
        for binding in &untyped {
            read_by_untyped.extend(self.reads_of(self.binding(*binding)?.init)?);
        }
        read_by_untyped.extend(self.typing.read_by_faults.iter().copied());
        if !self.cycle_spans.is_empty() {
            for node in &self.nodes {
                self.flow_step();
                if let Op::Read(binding) = node.op
                    && self.cycle_spans.iter().any(|span| span.contains(node.loc))
                {
                    read_by_untyped.insert(binding);
                }
            }
        }
        let untyped_values: FastSet<NodeId> = self.typing.untyped_values.clone();
        let members: FastSet<Member> = remaining.iter().copied().collect();
        let mut writers = FastSet::default();
        let mut neighbours: FastMap<Member, Vec<Member>> = FastMap::default();
        let mut read_by_member = FastSet::default();
        for &member in remaining {
            self.flow_step();
            if self.writes_variant(self.member_node(member)?)? {
                writers.insert(member);
            }
            if let Member::Cell(binding) = member {
                for read in self.reads_of(self.binding(binding)?.init)? {
                    if read != binding && members.contains(&Member::Cell(read)) {
                        read_by_member.insert(read);
                    }
                }
            }
            let met = flows
                .values
                .get(&member)
                .into_iter()
                .flatten()
                .chain(flows.partners.get(&member).into_iter().flatten())
                .filter(|other| **other != member && members.contains(other))
                .copied()
                .collect::<Vec<_>>();
            for other in met {
                neighbours.entry(member).or_default().push(other);
                neighbours.entry(other).or_default().push(member);
            }
        }
        for &member in remaining {
            self.flow_step();
            self.mark_untyped(member)?;
        }
        let mut seen = FastSet::default();
        for &start in remaining {
            if !seen.insert(start) {
                continue;
            }
            let mut flow = vec![start];
            let mut stack = vec![start];
            while let Some(member) = stack.pop() {
                crate::semantics::counted_walk::CountedWalk::EnumBindingsFlowsWithoutEnum.step();
                self.flow_step();
                for other in neighbours.get(&member).into_iter().flatten() {
                    if seen.insert(*other) {
                        flow.push(*other);
                        stack.push(*other);
                    }
                }
            }
            let live = flow.iter().any(|member| match member {
                Member::Cell(binding) => {
                    self.typing.reads.contains_key(binding)
                        || (!read_by_member.contains(binding) && !read_by_untyped.contains(binding))
                }
                Member::Value(node) => flows.used.contains(node),
            });
            let written = flow.iter().any(|member| writers.contains(member));
            let unread = flow.iter().all(|member| {
                !blocked.contains(member)
                    && !matches!(member, Member::Cell(binding) if read_by_untyped.contains(binding))
                    && self
                        .member_expected(*member, flows, &FastMap::default())
                        .is_empty()
            });
            if written && !live && unread {
                self.unplaced_flow(&flow)?;
                continue;
            }
            let inherited = flow.iter().any(|member| {
                flows
                    .values
                    .get(member)
                    .into_iter()
                    .flatten()
                    .any(|read| match read {
                        Member::Cell(binding) => untyped.contains(binding),
                        Member::Value(node) => untyped_values.contains(node),
                    })
            });
            if !live || (!written && inherited) {
                continue;
            }
            let mut located = Vec::with_capacity(flow.len());
            for member in flow {
                self.flow_step();
                let (loc, scope) = self.member_site(member, &flows.lets)?;
                located.push((!writers.contains(&member), loc, member, scope));
            }
            located.sort();
            let Some(((_, loc, first, scope), others)) = located.split_first() else {
                continue;
            };
            let mut related: Vec<Loc> = others.iter().map(|(_, loc, _, _)| *loc).collect();
            for (_, writer, member, _) in &located {
                if let Member::Value(node) = member {
                    for alias in flows.lets.aliases.get(node).into_iter().flatten() {
                        self.flow_step();
                        if alias != writer && alias != loc {
                            related.push(*alias);
                        }
                    }
                }
            }
            related.retain(|other| other != loc);
            related.sort();
            related.dedup();
            let related = related
                .into_iter()
                .map(|other| (other, "another use of the value".to_string()))
                .collect();
            let at = self.value_loc(*first, &flows.lets)?;
            self.report_flow(*first, at, *scope, related)?;
        }
        Ok(())
    }

    fn unplaced_flow(&mut self, flow: &[Member]) -> Result<(), Internal> {
        let mut bares = Vec::new();
        for member in flow {
            if let Member::Value(node) = member {
                for bare in self.value_bares(*node)? {
                    let record = self.node(bare)?;
                    if let Op::Bare(name) = &record.op {
                        bares.push((bare, name.clone(), record.loc, record.scope));
                    }
                }
            }
        }
        bares.sort_by_key(|(bare, ..)| *bare);
        bares.dedup_by_key(|(bare, ..)| *bare);
        for (_, name, loc, scope) in bares {
            if !self.in_trial(scope)? {
                self.unplaced_variant_at(Some(&name), loc, scope)?;
            }
        }
        for member in flow {
            self.mark_untyped(*member)?;
        }
        Ok(())
    }

    fn report_flow(
        &mut self,
        member: Member,
        loc: Loc,
        scope: ScopeId,
        related: Vec<(Loc, String)>,
    ) -> Result<(), Internal> {
        self.mark_untyped(member)?;
        if self.in_probe(scope)? {
            return Ok(());
        }
        self.error_with_chain(
            Code::TypeMismatch,
            loc,
            related,
            scope,
            "this value names a variant, and its uses expect no single enum",
        )
    }

    fn writes_variant(&self, root: NodeId) -> Result<bool, Internal> {
        let mut seen = FastSet::default();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::EnumBindingsWritesVariant.step();
            self.flow_step();
            if !seen.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::Bare(_) => return Ok(true),
                Op::Read(_) | Op::Use(_) => {}
                op => stack.extend(op.operands()),
            }
        }
        Ok(false)
    }

    fn let_writers(&self) -> Result<LetWriters, Internal> {
        let mut aliases: FastMap<NodeId, Vec<Loc>> = FastMap::default();
        let mut values = Vec::with_capacity(self.memo.lets.len());
        for ((scope, local), bound) in &self.memo.lets {
            crate::semantics::counted_walk::CountedWalk::EnumBindingsLetWriters.step();
            self.flow_step();
            let Some(decl) = self.let_decl(*scope, *local)? else {
                continue;
            };
            values.push((self.hir.expr(decl.value)?.loc, decl.loc));
            let items: Vec<NodeId> = match bound {
                BoundVal::Scalar(Bound::Value(node)) => vec![*node],
                BoundVal::Array(array) => array
                    .1
                    .iter()
                    .filter_map(|item| match item {
                        Bound::Value(node) => Some(*node),
                        Bound::Cell(_) => None,
                    })
                    .collect(),
                BoundVal::Scalar(Bound::Cell(_)) | BoundVal::Error => Vec::new(),
            };
            let mut sources = Vec::with_capacity(items.len());
            for item in items {
                sources.push(self.through_uses(item)?);
            }
            sources.sort();
            sources.dedup();
            for source in sources {
                aliases.entry(source).or_default().push(decl.loc);
            }
        }
        for locs in aliases.values_mut() {
            locs.sort();
            locs.dedup();
        }
        values.sort();
        values.dedup();
        Ok(LetWriters { aliases, values })
    }
}

fn value_of(lets: &LetWriters, at: Loc) -> Option<Loc> {
    let after = lets
        .values
        .partition_point(|(value, _)| (value.file, value.start) <= (at.file, at.start));
    let (value, _) = lets.values.get(after.checked_sub(1)?)?;
    value.contains(at).then_some(*value)
}

fn writer_of(lets: &LetWriters, at: Loc) -> Option<Loc> {
    let after = lets
        .values
        .partition_point(|(value, _)| (value.file, value.start) <= (at.file, at.start));
    let (value, decl) = lets.values.get(after.checked_sub(1)?)?;
    value.contains(at).then_some(*decl)
}

fn is_input(member: Member, flows: &Flows) -> bool {
    matches!(member, Member::Cell(binding) if flows.inputs.contains(&binding))
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

struct FlowOrder {
    reads_first: Vec<Member>,
    readers_first: Vec<Member>,
    read_position: FastMap<Member, usize>,
    reader_position: FastMap<Member, usize>,
    neighbours: FastMap<Member, Vec<Member>>,
}

impl FlowOrder {
    fn new(reads_first: &[Member], readers_first: &[Member], flows: &Flows) -> FlowOrder {
        let mut neighbours: FastMap<Member, Vec<Member>> = FastMap::default();
        for (reader, reads) in &flows.values {
            for read in reads {
                neighbours.entry(*reader).or_default().push(*read);
                neighbours.entry(*read).or_default().push(*reader);
            }
        }
        for (member, partners) in &flows.partners {
            neighbours
                .entry(*member)
                .or_default()
                .extend(partners.iter().copied());
        }
        FlowOrder {
            reads_first: reads_first.to_vec(),
            readers_first: readers_first.to_vec(),
            read_position: reads_first
                .iter()
                .enumerate()
                .map(|(position, member)| (*member, position))
                .collect(),
            reader_position: readers_first
                .iter()
                .enumerate()
                .map(|(position, member)| (*member, position))
                .collect(),
            neighbours,
        }
    }
}

struct Dirty {
    reads: std::collections::BTreeSet<usize>,
    readers: std::collections::BTreeSet<usize>,
    partners: std::collections::BTreeSet<usize>,
}

impl Dirty {
    fn all(order: &FlowOrder) -> Dirty {
        Dirty {
            reads: (0..order.reads_first.len()).collect(),
            readers: (0..order.readers_first.len()).collect(),
            partners: (0..order.readers_first.len()).collect(),
        }
    }

    fn touch(&mut self, member: Member, order: &FlowOrder) {
        let around = order.neighbours.get(&member).into_iter().flatten();
        for touched in std::iter::once(&member).chain(around) {
            if let Some(position) = order.read_position.get(touched) {
                self.reads.insert(*position);
            }
            if let Some(position) = order.reader_position.get(touched) {
                self.readers.insert(*position);
                self.partners.insert(*position);
            }
        }
    }
}
