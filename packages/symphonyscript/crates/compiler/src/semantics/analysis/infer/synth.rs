use symphonyscript_core::{BinaryOp, Builtin, Scalar, UnaryOp};

use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::sig::{Kind, Sig, Width, common, float_for, is_integer};

impl Engine<'_> {
    pub(super) fn synth(&mut self, root: NodeId) -> Option<Scalar> {
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            if self.typing.synth.contains_key(&node) {
                continue;
            }
            let sig = self.memo.sigs.get(&node).copied().unwrap_or(Sig::ERROR);
            if !sig.reactive {
                let ty = match sig.width {
                    Width::Fixed(scalar) => Some(scalar),
                    Width::Widened(scalar) => Some(float_for(scalar)),
                    _ => None,
                };
                self.typing.synth.insert(node, ty);
                continue;
            }
            let op = self.node(node).op.clone();
            let operands = match &op {
                Op::Read(_) => Vec::new(),
                other => other.operands(),
            };
            if !ready {
                stack.push((node, true));
                stack.extend(
                    operands
                        .iter()
                        .filter(|operand| !self.typing.synth.contains_key(operand))
                        .map(|operand| (*operand, false)),
                );
                continue;
            }
            let ty = self.synth_of(&op, &sig);
            self.typing.synth.insert(node, ty);
        }
        self.typing.synth.get(&root).copied().flatten()
    }

    fn known(&self, node: NodeId) -> (Option<Scalar>, Kind) {
        let kind = self
            .memo
            .sigs
            .get(&node)
            .map_or(Kind::Error, |sig| sig.kind);
        (self.typing.synth.get(&node).copied().flatten(), kind)
    }

    pub(super) fn combine(&self, operands: &[NodeId]) -> Option<Scalar> {
        let mut known: Option<Scalar> = None;
        let mut flexible_float = false;
        for operand in operands {
            match self.known(*operand) {
                (Some(scalar), _) => {
                    known = Some(match known {
                        Some(current) => common(current, scalar).unwrap_or(current),
                        None => scalar,
                    });
                }
                (None, Kind::Float) => flexible_float = true,
                _ => {}
            }
        }
        match known {
            Some(scalar) if flexible_float && is_integer(scalar) => Some(float_for(scalar)),
            other => other,
        }
    }

    fn synth_of(&mut self, op: &Op, sig: &Sig) -> Option<Scalar> {
        match sig.kind {
            Kind::Bool => return Some(Scalar::Bool),
            Kind::Enum(id) => return Some(Scalar::Enum(id)),
            Kind::Str => return Some(Scalar::Str),
            _ => {}
        }
        match op {
            Op::Read(binding) => self.typing.own.get(binding).copied(),
            Op::Use(inner) | Op::Unary(UnaryOp::Neg, inner) => self.known(*inner).0,
            Op::Unary(UnaryOp::Not, _) => Some(Scalar::Bool),
            Op::Binary(op, lhs, rhs) => {
                let combined = self.combine(&[*lhs, *rhs]);
                match op {
                    BinaryOp::Div => combined.map(float_for),
                    _ => combined,
                }
            }
            Op::If(_, then, otherwise) => self.combine(&[*then, *otherwise]),
            Op::Match {
                arms, otherwise, ..
            } => {
                let mut values: Vec<NodeId> = arms.iter().map(|(_, value)| *value).collect();
                values.extend(otherwise.iter().copied());
                self.combine(&values)
            }
            Op::Pick { items, .. } => self.combine(items),
            Op::Convert(target, _) | Op::Annotate(target, _) => Some(*target),
            Op::Builtin(builtin, args) => match builtin {
                Builtin::Abs | Builtin::Min | Builtin::Max | Builtin::Clamp => self.combine(args),
                Builtin::Rand => Some(Scalar::F32),
                _ => self.combine(args).map(float_for),
            },
            Op::ToRead { attrs, .. } => attrs
                .first()
                .and_then(|(item, attr)| self.attr_decl(*item, *attr))
                .map(|decl| decl.ty.scalar),
            _ => None,
        }
    }
}
