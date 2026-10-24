use symphonyscript_core::Scalar;

use crate::diagnostics::Code;
use crate::semantics::analysis::binding::BindingKind;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::BindingId;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::sig::{Kind, Width, is_float, is_integer, rank};

impl Engine<'_> {
    pub(super) fn own_type(&mut self, binding: BindingId) -> Option<Scalar> {
        let record = self.binding(binding).clone();
        match record.kind {
            BindingKind::Input { .. } => return self.input_type(binding),
            BindingKind::Attr { .. } => return record.annotation,
            BindingKind::Let | BindingKind::Param => {}
        }
        if let Some(annotation) = record.annotation {
            return Some(annotation);
        }
        let sig = self.demand_sig(record.init);
        let expectations = self
            .typing
            .expectations
            .get(&binding)
            .cloned()
            .unwrap_or_default();
        match sig.kind {
            Kind::Bool => Some(Scalar::Bool),
            Kind::Enum(id) => Some(Scalar::Enum(id)),
            Kind::Bare => {
                let enums: Vec<Scalar> = expectations
                    .iter()
                    .map(|(scalar, _)| *scalar)
                    .filter(|scalar| matches!(scalar, Scalar::Enum(_)))
                    .collect();
                match enums.first() {
                    Some(first) if enums.iter().all(|scalar| scalar == first) => Some(*first),
                    _ => {
                        self.error(
                            Code::TypeMismatch,
                            record.loc,
                            self.node(record.init).scope,
                            "this value names a variant, and its uses expect no single enum",
                        );
                        None
                    }
                }
            }
            Kind::Str => {
                self.error(
                    Code::ReactiveString,
                    record.loc,
                    self.node(record.init).scope,
                    "a string exists only at compile time, and this one reads a cell",
                );
                None
            }
            Kind::Node => {
                self.error(
                    Code::ReactiveNodeReference,
                    record.loc,
                    self.node(record.init).scope,
                    "a value holding node references reads no input; write the choice in the destination",
                );
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
                    None => Some(self.natural(binding).unwrap_or(Scalar::I32)),
                }
            }
            Kind::Any | Kind::Error => None,
        }
    }

    fn natural(&mut self, root: BindingId) -> Option<Scalar> {
        let mut stack = vec![root];
        while let Some(&binding) = stack.last() {
            if self.typing.natural.contains_key(&binding) {
                stack.pop();
                continue;
            }
            let record = self.binding(binding).clone();
            let fixed = match record.kind {
                BindingKind::Input { .. } => Some(self.input_type(binding)),
                BindingKind::Attr { .. } => Some(record.annotation),
                BindingKind::Let | BindingKind::Param => record.annotation.map(Some),
            };
            if let Some(fixed) = fixed {
                let fixed = fixed.filter(|scalar| is_integer(*scalar));
                self.typing.natural.insert(binding, fixed);
                stack.pop();
                continue;
            }
            let reads = self.reads_of(record.init);
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
                let contribution = match &self.node(node).op {
                    Op::Read(read) => self.typing.natural.get(read).copied().flatten(),
                    Op::Number(..)
                    | Op::Const(_)
                    | Op::Bool(_)
                    | Op::Str(_)
                    | Op::Concat(_)
                    | Op::Variant(..)
                    | Op::Bare(_)
                    | Op::Pi
                    | Op::NodeRef(_)
                    | Op::Use(_)
                    | Op::Arg(_)
                    | Op::Unary(..)
                    | Op::Binary(..)
                    | Op::If(..)
                    | Op::Match { .. }
                    | Op::Pick { .. }
                    | Op::Convert(..)
                    | Op::Annotate(..)
                    | Op::Builtin(..)
                    | Op::Recursive { .. }
                    | Op::ToRead { .. } => match self.memo.sigs.get(&node).map(|sig| sig.width) {
                        Some(Width::Fixed(scalar)) if is_integer(scalar) => Some(scalar),
                        Some(
                            Width::Fixed(_) | Width::Flexible | Width::Widened(_) | Width::Adaptive,
                        )
                        | None => None,
                    },
                };
                if let Some(contribution) = contribution
                    && widest.is_none_or(|current| rank(contribution) > rank(current))
                {
                    widest = Some(contribution);
                }
                if !matches!(self.node(node).op, Op::Read(_)) {
                    nodes.extend(self.node(node).op.operands());
                }
            }
            self.typing.natural.insert(binding, widest);
            stack.pop();
        }
        self.typing.natural.get(&root).copied().flatten()
    }
}
