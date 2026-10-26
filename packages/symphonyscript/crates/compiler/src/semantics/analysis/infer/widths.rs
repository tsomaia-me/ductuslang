use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::is_float::is_float;
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
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
                    "a value holding node references reads no input; write the choice in the destination",
                )?;
                None
            }
            Kind::Float => {
                let widest = expectations
                    .iter()
                    .map(|(scalar, _)| *scalar)
                    .filter(|scalar| is_float(*scalar))
                    .max_by_key(|scalar| rank(*scalar));
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
                    None => Some(self.natural(binding)?.unwrap_or(Scalar::I32)),
                }
            }
            Kind::Any | Kind::Error => None,
        })
    }

    fn natural(&mut self, root: BindingId) -> Result<Option<Scalar>, Internal> {
        let mut stack = vec![root];
        while let Some(&binding) = stack.last() {
            if self.typing.natural.contains_key(&binding) {
                stack.pop();
                continue;
            }
            let record = self.binding(binding)?.clone();
            let fixed = match record.kind {
                BindingKind::Input { .. } => Some(self.input_type(binding)?),
                BindingKind::Attr { .. } => Some(record.annotation),
                BindingKind::Let | BindingKind::Param | BindingKind::State { .. } => {
                    record.annotation.map(Some)
                }
            };
            if let Some(fixed) = fixed {
                let fixed = fixed.filter(|scalar| is_integer(*scalar));
                self.typing.natural.insert(binding, fixed);
                stack.pop();
                continue;
            }
            let reads = self.reads_of(record.init)?;
            let pending: Vec<BindingId> = reads
                .iter()
                .copied()
                .filter(|read| !self.typing.natural.contains_key(read))
                .collect();
            if !pending.is_empty() {
                stack.extend(pending);
                continue;
            }
            let mut widest: Option<Scalar> = None;
            let mut nodes = vec![record.init];
            let mut seen = std::collections::HashSet::new();
            while let Some(node) = nodes.pop() {
                if !seen.insert(node) {
                    continue;
                }
                self.memo.read_walk += 1;
                let read_free = !self.reaches_read(node);
                let contribution = match &self.node(node)?.op {
                    _ if read_free => self.read_free_width(node)?,
                    Op::Read(read) => self.natural_of(*read)?,
                    _ => self.fixed_integer_width(node)?,
                };
                widest = wider(widest, contribution);
                if !read_free && !matches!(self.node(node)?.op, Op::Read(_)) {
                    nodes.extend(self.node(node)?.op.operands());
                }
            }
            self.typing.natural.insert(binding, widest);
            stack.pop();
        }
        self.natural_of(root)
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
