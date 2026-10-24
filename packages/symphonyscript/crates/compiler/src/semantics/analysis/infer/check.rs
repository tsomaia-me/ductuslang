use symphonyscript_core::{BinaryOp, Builtin, Scalar, UnaryOp};

use crate::diagnostics::Code;
use crate::semantics::analysis::cval::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::evaluation::own_type;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::sig::{Kind, Sig, Width, is_float, is_integer, widens};

impl Engine<'_> {
    pub(super) fn check(
        &mut self,
        root: NodeId,
        expected: Option<Scalar>,
        condition: bool,
        roots: &mut Vec<(NodeId, Option<Scalar>)>,
    ) {
        let mut stack = vec![(root, expected)];
        let mut condition_root = condition;
        while let Some((node, expected)) = stack.pop() {
            if !self.typing.checked.insert((node, expected)) {
                continue;
            }
            let sig = self.memo.sigs.get(&node).copied().unwrap_or(Sig::ERROR);
            if sig.is_error() {
                continue;
            }
            let as_condition = std::mem::take(&mut condition_root);
            if let Some(expected) = expected
                && !self.compatible(node, &sig, expected, as_condition)
            {
                continue;
            }
            if !sig.reactive {
                roots.push((node, expected));
                continue;
            }
            let ty = match expected {
                Some(expected) => Some(expected),
                None => self.synth(node),
            };
            let op = self.node(node).op.clone();
            match op {
                Op::Read(_) | Op::ToRead { .. } | Op::Concat(_) => {}
                Op::Use(inner) => stack.push((inner, ty)),
                Op::Unary(UnaryOp::Neg, operand) => stack.push((operand, ty)),
                Op::Unary(UnaryOp::Not, operand) => stack.push((operand, Some(Scalar::Bool))),
                Op::Binary(op, lhs, rhs) => match op {
                    BinaryOp::And | BinaryOp::Or => {
                        stack.push((lhs, Some(Scalar::Bool)));
                        stack.push((rhs, Some(Scalar::Bool)));
                    }
                    BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge
                    | BinaryOp::Is
                    | BinaryOp::IsNot => {
                        self.synth(lhs);
                        self.synth(rhs);
                        let pair = self.pair(lhs, rhs);
                        stack.push((lhs, pair));
                        stack.push((rhs, pair));
                    }
                    BinaryOp::IntDiv if ty.is_some_and(is_float) => {
                        self.synth(lhs);
                        self.synth(rhs);
                        let inner = self.combine(&[lhs, rhs]);
                        stack.push((lhs, inner));
                        stack.push((rhs, inner));
                    }
                    _ => {
                        stack.push((lhs, ty));
                        stack.push((rhs, ty));
                    }
                },
                Op::If(condition, then, otherwise) => {
                    stack.push((condition, Some(Scalar::Bool)));
                    match self.chosen(condition, EvalTy::Bool) {
                        Some(CVal::Bool(true)) => stack.push((then, ty)),
                        Some(CVal::Bool(false)) => stack.push((otherwise, ty)),
                        Some(_) => {}
                        None => {
                            stack.push((then, ty));
                            stack.push((otherwise, ty));
                        }
                    }
                }
                Op::Match {
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    let scrutinee_ty = self.synth(scrutinee);
                    stack.push((scrutinee, scrutinee_ty));
                    let label_ty = self
                        .memo
                        .sigs
                        .get(&scrutinee)
                        .map_or(EvalTy::Exact, |sig| own_type(sig, EvalTy::Exact));
                    match self.chosen(scrutinee, label_ty) {
                        Some(CVal::Error) => {}
                        Some(value) => {
                            let chosen = arms
                                .iter()
                                .find(|(label, _)| self.demand_cval(*label, label_ty) == value)
                                .map(|(_, arm)| *arm)
                                .or(otherwise);
                            stack.extend(chosen.map(|arm| (arm, ty)));
                        }
                        None => {
                            for (_, value) in arms {
                                stack.push((value, ty));
                            }
                            stack.extend(otherwise.map(|value| (value, ty)));
                        }
                    }
                }
                Op::Pick { index, items } => {
                    let index_ty = self.synth(index);
                    stack.push((index, index_ty));
                    let position_ty = self
                        .memo
                        .sigs
                        .get(&index)
                        .map_or(EvalTy::Exact, |sig| own_type(sig, EvalTy::Exact));
                    match self.chosen(index, position_ty) {
                        Some(CVal::Int(position)) => {
                            match usize::try_from(position)
                                .ok()
                                .and_then(|position| items.get(position))
                            {
                                Some(item) => stack.push((*item, ty)),
                                None => self.error_at_node(
                                    Code::IndexOutOfRange,
                                    index,
                                    format!(
                                        "the index {position} is outside the array of length {}",
                                        items.len()
                                    ),
                                ),
                            }
                        }
                        Some(_) => {}
                        None => stack.extend(items.into_iter().map(|item| (item, ty))),
                    }
                }
                Op::Convert(target, operand) => {
                    let operand_ty = if is_float(target) {
                        Some(target)
                    } else {
                        self.synth(operand)
                    };
                    stack.push((operand, operand_ty));
                }
                Op::Annotate(target, operand) => stack.push((operand, Some(target))),
                Op::Builtin(builtin, args) => {
                    let operands = match builtin {
                        Builtin::Rand => Some(Scalar::I64),
                        _ => ty,
                    };
                    stack.extend(args.into_iter().map(|arg| (arg, operands)));
                }
                other => {
                    for operand in other.operands() {
                        let operand_ty = self.synth(operand);
                        stack.push((operand, operand_ty));
                    }
                }
            }
        }
    }

    fn chosen(&mut self, node: NodeId, ty: EvalTy) -> Option<CVal> {
        let sig = self.memo.sigs.get(&node).copied().unwrap_or(Sig::ERROR);
        if sig.reactive || sig.is_error() {
            return None;
        }
        Some(self.demand_cval(node, ty))
    }

    fn pair(&mut self, lhs: NodeId, rhs: NodeId) -> Option<Scalar> {
        let left = self.memo.sigs.get(&lhs).copied().unwrap_or(Sig::ERROR);
        let right = self.memo.sigs.get(&rhs).copied().unwrap_or(Sig::ERROR);
        match (left.kind, right.kind) {
            (Kind::Bool, _) => return Some(Scalar::Bool),
            (Kind::Str, _) => return Some(Scalar::Str),
            (Kind::Enum(id), _) | (_, Kind::Enum(id)) => return Some(Scalar::Enum(id)),
            _ => {}
        }
        self.combine(&[lhs, rhs])
    }

    fn compatible(&mut self, node: NodeId, sig: &Sig, expected: Scalar, condition: bool) -> bool {
        let is_cell = matches!(self.node(self.through_uses(node)).op, Op::Read(_))
            || matches!(sig.width, Width::Adaptive)
                && matches!(self.node(node).op, Op::ToRead { .. });
        let fits_kind = match (sig.kind, expected) {
            (Kind::Bool, Scalar::Bool) | (Kind::Str, Scalar::Str) => true,
            (Kind::Enum(id), Scalar::Enum(other)) => id == other,
            (Kind::Bare, Scalar::Enum(_)) | (Kind::Any, _) => true,
            (Kind::Int, scalar) => is_integer(scalar) || is_float(scalar),
            (Kind::Float, scalar) if is_float(scalar) => true,
            (Kind::Float, scalar) if is_integer(scalar) => {
                if is_cell {
                    self.error_at_node(
                        Code::FloatToInteger,
                        node,
                        "a float cell is used where an integer is expected; a conversion states the rounding",
                    );
                } else {
                    self.error_at_node(
                        Code::TypeMismatch,
                        node,
                        "a float value stands where an integer is expected; a conversion states the rounding",
                    );
                }
                return false;
            }
            _ => false,
        };
        if !fits_kind && condition {
            self.error_at_node(Code::ConditionType, node, "a gate or a guard is a `bool`");
            return false;
        }
        if !fits_kind {
            self.error_at_node(
                Code::TypeMismatch,
                node,
                format!(
                    "this value has the wrong type here; `{}` is expected",
                    scalar_name(self, expected)
                ),
            );
            return false;
        }
        if let Width::Fixed(scalar) = sig.width
            && !is_cell
            && (is_integer(scalar) || is_float(scalar))
            && !widens(scalar, expected)
        {
            self.error_at_node(
                Code::TypeMismatch,
                node,
                format!(
                    "a `{}` value does not widen into `{}`",
                    scalar_name(self, scalar),
                    scalar_name(self, expected)
                ),
            );
            return false;
        }
        true
    }
}

pub(super) fn scalar_name(engine: &Engine<'_>, scalar: Scalar) -> String {
    match scalar {
        Scalar::I32 => "i32".into(),
        Scalar::I64 => "i64".into(),
        Scalar::F32 => "f32".into(),
        Scalar::F64 => "f64".into(),
        Scalar::Bool => "bool".into(),
        Scalar::Str => "string".into(),
        Scalar::Enum(id) => engine
            .vocabulary
            .enum_decl(id)
            .map_or(String::new(), |decl| decl.name.clone()),
    }
}
