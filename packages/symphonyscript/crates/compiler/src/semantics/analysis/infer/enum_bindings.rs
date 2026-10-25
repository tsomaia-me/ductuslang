use std::collections::VecDeque;

use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::op::Op;
use crate::semantics::ir::Loc;

impl Engine<'_> {
    pub(super) fn enum_bindings(&mut self) -> Result<(), Internal> {
        let pending = std::mem::take(&mut self.typing.awaiting_enum);
        let mut waiting: FastSet<BindingId> = pending.iter().copied().collect();
        let mut carried: FastMap<BindingId, Vec<BindingId>> = FastMap::default();
        let mut carriers: FastMap<BindingId, Vec<BindingId>> = FastMap::default();
        for &binding in &pending {
            let Some(reads) = self.carried_reads(self.binding(binding)?.init)? else {
                continue;
            };
            for read in &reads {
                carriers.entry(*read).or_default().push(binding);
            }
            carried.insert(binding, reads);
        }
        let mut queue: VecDeque<BindingId> = std::mem::take(&mut self.typing.enum_expected)
            .into_iter()
            .collect();
        let mut decided = Vec::new();
        while let Some(binding) = queue.pop_front() {
            if !waiting.remove(&binding) {
                continue;
            }
            let Some(own) = self.single_enum(binding)? else {
                self.no_single_enum(binding, Vec::new())?;
                continue;
            };
            decided.push(binding);
            self.take_enum(binding, own, &mut queue)?;
            let mut typed = vec![binding];
            while let Some(read) = typed.pop() {
                for carrier in carriers.get(&read).into_iter().flatten() {
                    let reads = carried.get(carrier).map(Vec::as_slice).unwrap_or_default();
                    let carries = reads
                        .iter()
                        .all(|read| self.typing.own.get(read) == Some(&own));
                    if carries && waiting.remove(carrier) {
                        self.take_enum(*carrier, own, &mut queue)?;
                        typed.push(*carrier);
                    }
                }
            }
        }
        for binding in decided {
            if self.single_enum(binding)?.is_none() {
                self.no_single_enum(binding, Vec::new())?;
            }
        }
        let remaining: Vec<BindingId> = pending
            .into_iter()
            .filter(|binding| waiting.contains(binding))
            .collect();
        self.flows_without_enum(&remaining)
    }

    fn take_enum(
        &mut self,
        binding: BindingId,
        own: Scalar,
        queue: &mut VecDeque<BindingId>,
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
            self.expect(partner, Some(own))?;
        }
        queue.extend(std::mem::take(&mut self.typing.enum_expected));
        Ok(())
    }

    fn carried_reads(&self, init: NodeId) -> Result<Option<Vec<BindingId>>, Internal> {
        let mut reads = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![init];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::Read(binding) => reads.push(*binding),
                Op::Use(inner) => stack.push(*inner),
                Op::If(_, then, otherwise) => stack.extend([*then, *otherwise]),
                Op::Match {
                    arms, otherwise, ..
                } => {
                    stack.extend(arms.iter().map(|(_, value)| *value));
                    stack.extend(*otherwise);
                }
                Op::Pick { items, .. } => stack.extend(items.iter().copied()),
                Op::Element { base, .. } => stack.push(*base),
                Op::Array(elements) => stack.extend(elements.iter().copied()),
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Bare(_)
                | Op::Pi
                | Op::NodeRef(_)
                | Op::Arg(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::Convert(..)
                | Op::Annotate(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::ToRead { .. } => return Ok(None),
            }
        }
        reads.sort();
        reads.dedup();
        Ok((!reads.is_empty()).then_some(reads))
    }

    fn flows_without_enum(&mut self, remaining: &[BindingId]) -> Result<(), Internal> {
        let mut read_by_untyped = FastSet::default();
        let untyped: Vec<BindingId> = self.typing.untyped.iter().copied().collect();
        for binding in untyped {
            read_by_untyped.extend(self.reads_of(self.binding(binding)?.init)?);
        }
        let members: FastSet<BindingId> = remaining.iter().copied().collect();
        let mut locs: FastMap<BindingId, Loc> = FastMap::default();
        let mut neighbours: FastMap<BindingId, Vec<BindingId>> = FastMap::default();
        let mut read_by_member = FastSet::default();
        for &binding in remaining {
            let record = self.binding(binding)?;
            let (loc, init) = (record.loc, record.init);
            locs.insert(binding, loc);
            let mut met = Vec::new();
            for read in self.reads_of(init)? {
                if read != binding && members.contains(&read) {
                    read_by_member.insert(read);
                    met.push(read);
                }
            }
            let partners = self
                .typing
                .equal_to
                .get(&binding)
                .cloned()
                .unwrap_or_default();
            for partner in partners {
                if let Op::Read(other) = self.node(self.through_uses(partner)?)?.op
                    && other != binding
                    && members.contains(&other)
                {
                    met.push(other);
                }
            }
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
            if !live {
                continue;
            }
            let mut located = Vec::with_capacity(flow.len());
            for binding in flow {
                let loc = locs
                    .get(&binding)
                    .copied()
                    .ok_or(Internal::missed("binding"))?;
                located.push((loc, binding));
            }
            located.sort();
            let Some(((_, first), others)) = located.split_first() else {
                continue;
            };
            let secondary = others.iter().map(|(loc, _)| *loc).collect();
            self.no_single_enum(*first, secondary)?;
        }
        Ok(())
    }

    fn single_enum(&self, binding: BindingId) -> Result<Option<Scalar>, Internal> {
        let enums: Vec<Scalar> = self
            .typing
            .expectations
            .get(&binding)
            .into_iter()
            .flatten()
            .map(|(scalar, _)| *scalar)
            .filter(|scalar| matches!(scalar, Scalar::Enum(_)))
            .collect();
        Ok(match enums.first() {
            Some(first) if enums.iter().all(|scalar| scalar == first) => Some(*first),
            Some(_) | None => None,
        })
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
