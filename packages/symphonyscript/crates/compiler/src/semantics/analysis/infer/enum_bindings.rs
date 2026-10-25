use std::collections::VecDeque;

use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::ids::BindingId;

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
                self.no_single_enum(binding)?;
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
                self.no_single_enum(binding)?;
            }
        }
        let mut read_by_untyped = FastSet::default();
        let untyped: Vec<BindingId> = self.typing.untyped.iter().copied().collect();
        for binding in untyped {
            read_by_untyped.extend(self.reads_of(self.binding(binding)?.init)?);
        }
        for binding in pending {
            if !waiting.contains(&binding) {
                continue;
            }
            let unread = !self.typing.reads.contains_key(&binding);
            if unread && read_by_untyped.contains(&binding) {
                self.typing.untyped.insert(binding);
            } else {
                self.no_single_enum(binding)?;
            }
            read_by_untyped.extend(self.reads_of(self.binding(binding)?.init)?);
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

    fn no_single_enum(&mut self, binding: BindingId) -> Result<(), Internal> {
        self.typing.untyped.insert(binding);
        self.typing.own.remove(&binding);
        let record = self.binding(binding)?;
        let (loc, init) = (record.loc, record.init);
        self.error(
            Code::TypeMismatch,
            loc,
            self.node(init)?.scope,
            "this value names a variant, and its uses expect no single enum",
        )
    }
}
