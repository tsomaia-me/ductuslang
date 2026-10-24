use symphonyscript_core::{BinaryOp, Builtin, Scalar, UnaryOp};

use crate::semantics::Internal;
use crate::semantics::analysis::common::common;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::float_for::float_for;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::width::Width;

impl Engine<'_> {
    pub(super) fn synth(&mut self, root: NodeId) -> Result<Option<Scalar>, Internal> {
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            if self.typing.synth.contains_key(&node) {
                continue;
            }
            let sig = self.known_sig(node)?;
            if !sig.reactive {
                let ty = match sig.width {
                    Width::Fixed(scalar) => Some(scalar),
                    Width::Widened(scalar) => Some(float_for(scalar)),
                    Width::Flexible | Width::Adaptive => None,
                };
                self.typing.synth.insert(node, ty);
                continue;
            }
            let op = self.node(node)?.op.clone();
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
            let ty = self.synth_of(&op, &sig)?;
            self.typing.synth.insert(node, ty);
        }
        Ok(self.typing.synth.get(&root).copied().flatten())
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

    fn synth_of(&mut self, op: &Op, sig: &Sig) -> Result<Option<Scalar>, Internal> {
        match sig.kind {
            Kind::Bool => return Ok(Some(Scalar::Bool)),
            Kind::Enum(id) => return Ok(Some(Scalar::Enum(id))),
            Kind::Str => return Ok(Some(Scalar::Str)),
            Kind::Int | Kind::Float | Kind::Node | Kind::Bare | Kind::Any | Kind::Error => {}
        }
        Ok(match op {
            Op::Read(binding) => self.typing.own.get(binding).copied(),
            Op::Use(inner) | Op::Unary(UnaryOp::Neg, inner) => self.known(*inner).0,
            Op::Unary(UnaryOp::Not, _) => Some(Scalar::Bool),
            Op::Binary(op, lhs, rhs) => {
                let combined = self.combine(&[*lhs, *rhs]);
                match op {
                    BinaryOp::Div => combined.map(float_for),
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::IntDiv
                    | BinaryOp::Rem
                    | BinaryOp::Pow
                    | BinaryOp::Is
                    | BinaryOp::IsNot
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge
                    | BinaryOp::And
                    | BinaryOp::Or => combined,
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
            Op::Pick { items, .. } | Op::Array(items) => self.combine(items),
            Op::Element { base, .. } => self.known(*base).0,
            Op::Convert(target, _) | Op::Annotate(target, _) => Some(*target),
            Op::Builtin(builtin, args) => match builtin {
                Builtin::Abs | Builtin::Min | Builtin::Max | Builtin::Clamp => self.combine(args),
                Builtin::Rand => Some(Scalar::F32),
                Builtin::Floor
                | Builtin::Ceil
                | Builtin::Round
                | Builtin::Sqrt
                | Builtin::Sin
                | Builtin::Cos
                | Builtin::Tan
                | Builtin::Exp
                | Builtin::Log
                | Builtin::Lerp => self.combine(args).map(float_for),
            },
            Op::ToRead { attrs, .. } => match attrs.first() {
                Some((item, attr)) => self.attr_decl(*item, *attr)?.map(|decl| decl.ty.scalar),
                None => None,
            },
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
            | Op::Recursive { .. } => None,
        })
    }
}
