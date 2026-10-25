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
            self.typing.own.insert(binding, own);
            decided.push(binding);
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

    fn flows_without_enum(&mut self, remaining: &[BindingId]) -> Result<(), Internal> {
        let mut read_by_untyped = FastSet::default();
        let untyped: Vec<BindingId> = self.typing.untyped.iter().copied().collect();
        for binding in untyped {
            read_by_untyped.extend(self.reads_of(self.binding(binding)?.init)?);
        }
        let members: FastSet<BindingId> = remaining.iter().copied().collect();
        let mut locs: FastMap<BindingId, Loc> = FastMap::default();
        let mut readers: FastMap<BindingId, Vec<BindingId>> = FastMap::default();
        let mut upstream: FastMap<BindingId, Vec<BindingId>> = FastMap::default();
        let mut sources = FastSet::default();
        for &binding in remaining {
            let record = self.binding(binding)?;
            let (loc, init) = (record.loc, record.init);
            locs.insert(binding, loc);
            let mut reads = self.reads_of(init)?;
            reads.sort();
            reads.dedup();
            reads.retain(|read| *read != binding && members.contains(read));
            if reads.is_empty() || self.writes_variant(init)? {
                sources.insert(binding);
            }
            for read in &reads {
                readers.entry(*read).or_default().push(binding);
            }
            upstream.insert(binding, reads);
        }
        for list in readers.values_mut() {
            list.sort_by_key(|reader| locs.get(reader).copied());
        }
        let mut live = FastSet::default();
        let mut stack: Vec<BindingId> = remaining
            .iter()
            .copied()
            .filter(|binding| {
                self.typing.reads.contains_key(binding)
                    || (!readers.contains_key(binding) && !read_by_untyped.contains(binding))
            })
            .collect();
        while let Some(binding) = stack.pop() {
            if live.insert(binding) {
                stack.extend(upstream.get(&binding).into_iter().flatten().copied());
            }
        }
        for &binding in remaining {
            self.typing.untyped.insert(binding);
        }
        for &source in remaining {
            if !sources.contains(&source) || !live.contains(&source) {
                continue;
            }
            let mut secondary = Vec::new();
            let mut seen = FastSet::default();
            let mut queue: VecDeque<BindingId> = readers
                .get(&source)
                .into_iter()
                .flatten()
                .copied()
                .collect();
            while let Some(reader) = queue.pop_front() {
                if !live.contains(&reader) || !seen.insert(reader) {
                    continue;
                }
                secondary.push(
                    locs.get(&reader)
                        .copied()
                        .ok_or(Internal::missed("binding"))?,
                );
                if !sources.contains(&reader) {
                    queue.extend(readers.get(&reader).into_iter().flatten().copied());
                }
            }
            self.no_single_enum(source, secondary)?;
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
