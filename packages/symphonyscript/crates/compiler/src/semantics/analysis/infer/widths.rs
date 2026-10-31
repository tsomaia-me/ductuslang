use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::answers::CellRead;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::is_float::is_float;
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::rank::rank;
use crate::semantics::analysis::width::Width;

impl Engine<'_> {
    pub(super) fn own_type(&mut self, binding: BindingId) -> Result<Option<Scalar>, Internal> {
        let record = self.binding(binding)?.clone();
        match record.kind {
            BindingKind::Input { .. } => {
                let own = self.input_type(binding)?;
                if own.is_none() && self.demand_sig(record.init)?.kind == Kind::Bare {
                    self.typing.awaiting_enum.push(binding);
                }
                return Ok(own);
            }
            BindingKind::Attr { .. } => return Ok(record.annotation),
            BindingKind::Let | BindingKind::Param | BindingKind::State { .. } => {}
        }
        if let Some(annotation) = record.annotation {
            return Ok(Some(annotation));
        }
        let sig = self.demand_sig(record.init)?;
        let expectations = self
            .typing
            .expectations
            .get(&binding)
            .cloned()
            .unwrap_or_default();
        Ok(match sig.kind {
            Kind::Bool => Some(Scalar::Bool),
            Kind::Enum(id) => Some(Scalar::Enum(id)),
            Kind::Bare => {
                self.typing.awaiting_enum.push(binding);
                None
            }
            Kind::Str => {
                self.typing.untyped.insert(binding);
                self.error(
                    Code::ReactiveString,
                    record.loc,
                    self.node(record.init)?.scope,
                    "a string exists only at compile time, and this one reads a cell",
                )?;
                None
            }
            Kind::Node => {
                self.typing.untyped.insert(binding);
                self.error(
                    Code::ReactiveNodeReference,
                    record.loc,
                    self.node(record.init)?.scope,
                    "a value holding node references reads no cell; write the choice in the destination",
                )?;
                None
            }
            Kind::Float => {
                let widest = expectations
                    .iter()
                    .map(|(scalar, _)| *scalar)
                    .filter(|scalar| is_float(*scalar))
                    .max_by_key(|scalar| rank(*scalar));
                if widest.is_none() {
                    self.typing.default_widths.insert(binding);
                }
                Some(widest.unwrap_or(Scalar::F32))
            }
            Kind::Int => {
                let widest = expectations
                    .iter()
                    .map(|(scalar, _)| *scalar)
                    .filter(|scalar| is_integer(*scalar))
                    .max_by_key(|scalar| rank(*scalar));
                match widest {
                    Some(widest) => Some(widest),
                    None => {
                        self.typing.default_widths.insert(binding);
                        Some(self.natural(binding)?.unwrap_or(Scalar::I32))
                    }
                }
            }
            Kind::Any | Kind::Error => None,
        })
    }

    fn natural(&mut self, root: BindingId) -> Result<Option<Scalar>, Internal> {
        if !self.typing.natural.contains_key(&root) {
            self.settle_natural(root)?;
        }
        self.natural_of(root)
    }

    fn settle_natural(&mut self, root: BindingId) -> Result<(), Internal> {
        let mut parts: FastMap<BindingId, (Option<Scalar>, Vec<BindingId>)> = FastMap::default();
        let mut members = Vec::new();
        let mut pending = vec![root];
        while let Some(binding) = pending.pop() {
            crate::semantics::counted_walk::CountedWalk::WidthsSettleNatural.step();
            if self.typing.natural.contains_key(&binding) || parts.contains_key(&binding) {
                continue;
            }
            let found = self.natural_parts(binding)?;
            pending.extend(found.1.iter().copied());
            parts.insert(binding, found);
            members.push(binding);
        }
        let mut readers: FastMap<BindingId, Vec<BindingId>> = FastMap::default();
        let mut current: FastMap<BindingId, Option<Scalar>> = FastMap::default();
        for member in &members {
            let (own, cells) = parts.get(member).ok_or(Internal::missed("natural width"))?;
            current.insert(*member, *own);
            for cell in cells {
                readers.entry(*cell).or_default().push(*member);
            }
        }
        let mut work = members.clone();
        while let Some(member) = work.pop() {
            crate::semantics::counted_walk::CountedWalk::WidthsSettleNatural.step();
            let (own, cells) = parts
                .get(&member)
                .ok_or(Internal::missed("natural width"))?;
            let mut widest = *own;
            for cell in cells {
                let width = match current.get(cell) {
                    Some(width) => *width,
                    None => self.natural_of(*cell)?,
                };
                widest = wider(widest, width);
            }
            if current.get(&member) != Some(&widest) {
                current.insert(member, widest);
                work.extend(readers.get(&member).into_iter().flatten().copied());
            }
        }
        self.typing.natural.extend(current);
        Ok(())
    }

    fn natural_parts(
        &mut self,
        binding: BindingId,
    ) -> Result<(Option<Scalar>, Vec<BindingId>), Internal> {
        let record = self.binding(binding)?.clone();
        let fixed = match record.kind {
            BindingKind::Input { .. } => Some(self.input_type(binding)?),
            BindingKind::Attr { .. } => Some(record.annotation),
            BindingKind::Let | BindingKind::Param | BindingKind::State { .. } => {
                record.annotation.map(Some)
            }
        };
        if let Some(fixed) = fixed {
            return Ok((fixed.filter(|scalar| is_integer(*scalar)), Vec::new()));
        }
        self.mark_reaching_reads()?;
        let mut widest: Option<Scalar> = None;
        let mut cells = Vec::new();
        let mut nodes = vec![record.init];
        let mut seen = FastSet::default();
        while let Some(node) = nodes.pop() {
            if !seen.insert(node) {
                continue;
            }
            self.memo.read_walk += 1;
            if !self.reaches_read(node) {
                widest = wider(widest, self.read_free_width(node)?);
                continue;
            }
            match self.cell_read(node)? {
                CellRead::Binding(read) => cells.push(read),
                CellRead::History { .. } if !self.memo.own_history.contains(&node) => {
                    cells.extend(self.history_bindings(node)?);
                }
                CellRead::History { .. } | CellRead::Named(_) | CellRead::Operands(_) => {
                    nodes.extend(self.node(node)?.op.operands());
                    widest = wider(widest, self.fixed_integer_width(node)?);
                }
            }
        }
        Ok((widest, cells))
    }

    fn fixed_integer_width(&self, node: NodeId) -> Result<Option<Scalar>, Internal> {
        Ok(match self.known_sig(node)?.width {
            Width::Fixed(scalar) if is_integer(scalar) => Some(scalar),
            Width::Fixed(_) | Width::Flexible | Width::Widened(_) | Width::Adaptive => None,
        })
    }

    fn read_free_width(&mut self, root: NodeId) -> Result<Option<Scalar>, Internal> {
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            if self.memo.read_free_widths.contains_key(&node) {
                continue;
            }
            let operands = self.node(node)?.op.operands();
            if !ready {
                stack.push((node, true));
                stack.extend(operands.into_iter().map(|operand| (operand, false)));
                continue;
            }
            self.memo.read_walk += 1;
            let mut widest = self.fixed_integer_width(node)?;
            for operand in operands {
                let width = self
                    .memo
                    .read_free_widths
                    .get(&operand)
                    .copied()
                    .ok_or(Internal::missed("natural width"))?;
                widest = wider(widest, width);
            }
            self.memo.read_free_widths.insert(node, widest);
        }
        self.memo
            .read_free_widths
            .get(&root)
            .copied()
            .ok_or(Internal::missed("natural width"))
    }

    fn natural_of(&self, binding: BindingId) -> Result<Option<Scalar>, Internal> {
        self.typing
            .natural
            .get(&binding)
            .copied()
            .ok_or(Internal::missed("natural width"))
    }
}

fn wider(current: Option<Scalar>, other: Option<Scalar>) -> Option<Scalar> {
    match (current, other) {
        (Some(current), Some(other)) if rank(other) > rank(current) => Some(other),
        (None, other) => other,
        (current, _) => current,
    }
}
