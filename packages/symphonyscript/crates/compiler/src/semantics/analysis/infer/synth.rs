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
        self.synthesized(root)
    }

    fn synthesized(&self, node: NodeId) -> Result<Option<Scalar>, Internal> {
        self.typing
            .synth
            .get(&node)
            .copied()
            .ok_or(Internal::missed("synthesized type"))
    }

    fn known(&self, node: NodeId) -> Result<(Option<Scalar>, Kind), Internal> {
        Ok((self.synthesized(node)?, self.known_sig(node)?.kind))
    }

    fn history_bindings_type(&self, op: &Op) -> Result<Option<Scalar>, Internal> {
        let Op::History { cell, .. } = op else {
            return Ok(None);
        };
        use crate::semantics::analysis::bound::Bound;
        use crate::semantics::analysis::bound_val::BoundVal;
        use crate::semantics::analysis::history_cell::HistoryCell;
        use crate::semantics::analysis::let_ref::LetRef;
        let bound = match cell {
            HistoryCell::Input(input) => self.memo.inputs.get(input),
            HistoryCell::Let(id) => self
                .memo
                .lets
                .get(&(self.global_of_let(*id), LetRef::Top(*id))),
            HistoryCell::State(state) => self.memo.states.get(state),
        };
        let binding = match bound {
            Some(BoundVal::Scalar(Bound::Cell(binding))) => Some(*binding),
            Some(BoundVal::Array(array)) => array.1.iter().find_map(|item| match item {
                Bound::Cell(binding) => Some(*binding),
                Bound::Value(_) => None,
            }),
            Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => None,
        };
        Ok(binding.and_then(|binding| self.typing.own.get(&binding).copied()))
    }

    pub(super) fn combine(&self, operands: &[NodeId]) -> Result<Option<Scalar>, Internal> {
        let mut known: Option<Scalar> = None;
        let mut flexible_float = false;
        for operand in operands {
            match self.known(*operand)? {
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
        Ok(match known {
            Some(scalar) if flexible_float && is_integer(scalar) => Some(float_for(scalar)),
            other => other,
        })
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
            Op::Use(inner) | Op::Chosen(inner) | Op::Unary(UnaryOp::Neg, inner) => {
                self.known(*inner)?.0
            }
            Op::Unary(UnaryOp::Not, _) => Some(Scalar::Bool),
            Op::Binary(op, lhs, rhs) => {
                let combined = self.combine(&[*lhs, *rhs])?;
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
            Op::If(_, then, otherwise) => self.combine(&[*then, *otherwise])?,
            Op::Match {
                arms, otherwise, ..
            } => {
                let mut values: Vec<NodeId> = arms.iter().map(|(_, value)| *value).collect();
                values.extend(otherwise.iter().copied());
                self.combine(&values)?
            }
            Op::Pick { items, .. } | Op::Array(items) => self.combine(items)?,
            Op::Element { base, .. } => self.known(*base)?.0,
            Op::Convert(target, _) | Op::Annotate(target, _) => Some(*target),
            Op::Builtin(builtin, args) => match builtin {
                Builtin::Abs | Builtin::Min | Builtin::Max | Builtin::Clamp => {
                    self.combine(args)?
                }
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
                | Builtin::Lerp => self.combine(args)?.map(float_for),
                Builtin::Len => Some(Scalar::I32),
                Builtin::Bytes => Some(Scalar::U8),
            },
            Op::Join(lhs, rhs) => self.combine(&[*lhs, *rhs])?,
            Op::Slice { base, .. } => self.known(*base)?.0,
            Op::History { fallback, .. } => {
                let node_ty = self.history_bindings_type(op)?;
                match (node_ty, fallback) {
                    (Some(scalar), _) => Some(scalar),
                    (None, Some(fallback)) => self.known(*fallback)?.0,
                    (None, None) => None,
                }
            }
            Op::Native { native, .. } => {
                let decl = self
                    .vocabulary
                    .native(*native)
                    .ok_or(Internal::missed("native function"))?;
                Some(
                    crate::semantics::analysis::evaluation::native_ty(&decl.signature.result)
                        .scalar,
                )
            }
            Op::ToRead { attrs, .. } => {
                let (item, attr) = attrs.first().ok_or(Internal::missed("attribute"))?;
                self.attr_decl(*item, *attr)?.map(|decl| decl.ty.scalar)
            }
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
