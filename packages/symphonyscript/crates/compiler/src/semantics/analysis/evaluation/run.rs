use std::rc::Rc;

use symphonyscript_core::{BinaryOp, Builtin, SpecFloat, UnaryOp, Value};

use super::exact::{exact_binary, fits, represents};
use super::format::text;
use super::types::{
    Pair, int_bits, is_float_builtin, is_float_ty, is_int_ty, of_scalar, own_type, pair_type,
};
use crate::diagnostics::Code;
use crate::semantics::analysis::bound::{Bound, BoundVal};
use crate::semantics::analysis::cval::CVal;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::node::{Op, Piece};
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::{Kind, Width};
use crate::semantics::ir::{FloatLit, Number, Suffix};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Ctx {
    Global,
    Frame(usize),
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_eval(
        &mut self,
        node: NodeId,
        ty: EvalTy,
        need: &mut Vec<Query>,
    ) -> bool {
        match self.evaluate(node, ty, Ctx::Global, need) {
            Some(value) => {
                self.memo.evals.insert((node, ty), value);
                true
            }
            None => false,
        }
    }

    pub(super) fn evaluate(
        &mut self,
        node: NodeId,
        ty: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Option<CVal> {
        let sig = self.sig(node, need)?;
        if sig.is_error() || sig.reactive {
            return Some(CVal::Error);
        }
        let own = own_type(&sig, ty);
        if let Width::Widened(integer) = sig.width
            && let Some(scale) = self.memo.default_scales.get(&node).copied()
        {
            let value = self.compute(node, of_scalar(integer), ctx, need)?;
            return Some(convert_to(scaled_into(value, scale, own), ty));
        }
        let value = if own == EvalTy::Default && sig.kind == Kind::Float {
            match self.child(node, EvalTy::F64, ctx, need)? {
                CVal::F64(value) if !value.is_nan() && !(value as f32).is_finite() => {
                    CVal::F64(value)
                }
                CVal::F64(_) => self.child(node, EvalTy::F32, ctx, need)?,
                other => other,
            }
        } else {
            let value = self.compute(node, own, ctx, need)?;
            self.scaled(node, value)
        };
        Some(convert_to(value, ty))
    }

    pub(super) fn child(
        &mut self,
        node: NodeId,
        ty: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Option<CVal> {
        let Ctx::Frame(frame) = ctx else {
            return self.cval(Query::Eval(node, ty), need);
        };
        let local = self.memo.sigs.get(&node).is_some_and(|sig| sig.frame);
        if !local {
            return self.cval(Query::Eval(node, ty), need);
        }
        if let Some(value) = self.frame_value(frame, node, ty) {
            return Some(value);
        }
        self.frame_need.push((node, ty));
        None
    }

    fn compute(
        &mut self,
        node: NodeId,
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Option<CVal> {
        let op = self.node(node).op.clone();
        Some(match op {
            Op::Read(_) => CVal::Error,
            Op::Pick { index, items } => {
                let index_sig = self.sig(index, need)?;
                let index_ty = own_type(&index_sig, EvalTy::Exact);
                let position = match self.child(index, index_ty, ctx, need)? {
                    CVal::Int(position) => position,
                    _ => return Some(CVal::Error),
                };
                let item = usize::try_from(position)
                    .ok()
                    .and_then(|position| items.get(position).copied());
                match item {
                    Some(item) => self.child(item, own, ctx, need)?,
                    None => {
                        self.error_at_node(
                            Code::IndexOutOfRange,
                            index,
                            format!(
                                "the index {position} is outside the array of length {}",
                                items.len()
                            ),
                        );
                        CVal::Error
                    }
                }
            }
            Op::Arg(slot) => match ctx {
                Ctx::Frame(frame) => self.frame_arg(frame, slot),
                Ctx::Global => CVal::Error,
            },
            Op::Number(number, suffix) => self.literal(node, &number, suffix, own),
            Op::Const(value) => match value {
                Value::I32(value) => CVal::Int(i128::from(value)),
                Value::I64(value) => CVal::Int(i128::from(value)),
                Value::F32(value) => CVal::F32(value),
                Value::F64(value) => CVal::F64(value),
                Value::Bool(value) => CVal::Bool(value),
                Value::Enum(id, index) => CVal::Enum(id, index),
                Value::Str(_) => CVal::Error,
            },
            Op::Use(target) => self.child(target, own, ctx, need)?,
            Op::Bool(value) => CVal::Bool(value),
            Op::Str(text) => CVal::Str(Rc::from(text.as_str())),
            Op::Variant(id, index) => CVal::Enum(id, index),
            Op::NodeRef(item) => CVal::Node(item),
            Op::Pi => match own {
                EvalTy::F32 => CVal::F32(std::f32::consts::PI),
                _ => CVal::F64(std::f64::consts::PI),
            },
            Op::Bare(name) => match own {
                EvalTy::Enum(id) => {
                    let position = self
                        .vocabulary
                        .enum_decl(id)
                        .and_then(|decl| decl.variants.iter().position(|variant| *variant == name))
                        .and_then(|position| u16::try_from(position).ok());
                    match position {
                        Some(index) => CVal::Enum(id, index),
                        None => {
                            let enum_name = self
                                .vocabulary
                                .enum_decl(id)
                                .map_or(String::new(), |decl| decl.name.clone());
                            self.error_at_node(
                                Code::UnknownName,
                                node,
                                format!("`{name}` is not a variant of `{enum_name}`"),
                            );
                            CVal::Error
                        }
                    }
                }
                _ => CVal::Error,
            },
            Op::Concat(pieces) => return self.concat(&pieces, ctx, need),
            Op::Unary(UnaryOp::Not, operand) => {
                match self.child(operand, EvalTy::Bool, ctx, need)? {
                    CVal::Bool(value) => CVal::Bool(!value),
                    _ => CVal::Error,
                }
            }
            Op::Unary(UnaryOp::Neg, operand) => {
                let value = self.child(operand, own, ctx, need)?;
                match value {
                    CVal::Int(value) => self.integer(node, own, value.checked_neg()),
                    CVal::F32(value) => CVal::F32(-value),
                    CVal::F64(value) => CVal::F64(-value),
                    _ => CVal::Error,
                }
            }
            Op::Binary(op, _, _) => return self.binary(node, op, own, ctx, need),
            Op::If(condition, then, otherwise) => {
                let chosen = match self.child(condition, EvalTy::Bool, ctx, need)? {
                    CVal::Bool(true) => then,
                    CVal::Bool(false) => otherwise,
                    _ => return Some(CVal::Error),
                };
                self.child(chosen, own, ctx, need)?
            }
            Op::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let scrutinee_sig = self.sig(scrutinee, need)?;
                let scrutinee_ty = own_type(&scrutinee_sig, EvalTy::Exact);
                let value = self.child(scrutinee, scrutinee_ty, ctx, need)?;
                let mut chosen = otherwise;
                for (label, arm) in &arms {
                    let label_value = self.child(*label, scrutinee_ty, ctx, need)?;
                    if label_value == value {
                        chosen = Some(*arm);
                        break;
                    }
                }
                match chosen {
                    Some(arm) => self.child(arm, own, ctx, need)?,
                    None => CVal::Error,
                }
            }
            Op::Convert(_, operand) => {
                let operand_ty = if is_float_ty(own) {
                    own
                } else {
                    EvalTy::Default
                };
                let value = self.child(operand, operand_ty, ctx, need)?;
                convert(value, own)
            }
            Op::Annotate(_, operand) => {
                let value = self.child(operand, own, ctx, need)?;
                self.fit(node, value, own)
            }
            Op::Builtin(builtin, args) => {
                return self.builtin(node, builtin, &args, own, ctx, need);
            }
            Op::Recursive { .. } => return self.recursive(node, own, ctx, need),
            Op::ToRead { wire, attrs, .. } => {
                let destination = self.destination(Query::Destination(wire), need)?;
                let Destination::Fixed(item) = destination else {
                    return Some(CVal::Error);
                };
                let Some((_, attr)) = attrs.iter().find(|(candidate, _)| *candidate == item) else {
                    return Some(CVal::Error);
                };
                match self.bound(Query::Attr(item, *attr), need)? {
                    BoundVal::Scalar(Bound::Value(value)) => self.child(value, own, ctx, need)?,
                    _ => CVal::Error,
                }
            }
        })
    }

    fn literal(&mut self, node: NodeId, number: &Number, suffix: Suffix, own: EvalTy) -> CVal {
        let scale = match suffix {
            Suffix::Unit { family, unit } => self
                .vocabulary
                .family(family)
                .and_then(|decl| decl.units.get(unit))
                .map(|unit| unit.scale),
            _ => self
                .memo
                .default_scales
                .get(&node)
                .copied()
                .filter(|scale| is_float_ty(own) || scale.fract() == 0.0),
        };
        if is_int_ty(own) {
            let Number::Int(value) = number else {
                return CVal::Error;
            };
            let value = match scale {
                Some(scale) if scale.fract() == 0.0 && scale.abs() < 1e38 => {
                    value.checked_mul(scale as i128)
                }
                Some(_) => return CVal::Error,
                None => Some(*value),
            };
            let Some(value) = value.filter(|value| *value != i128::MIN) else {
                self.error_at_node(
                    Code::IntegerOverflow,
                    node,
                    "this value's magnitude reaches 2¹²⁷",
                );
                return CVal::Error;
            };
            if let Some(bits) = int_bits(own)
                && !fits(value, bits)
            {
                self.error_at_node(
                    Code::DoesNotFit,
                    node,
                    format!("the value {value} does not fit `{}`", ty_name(own)),
                );
                return CVal::Error;
            }
            return CVal::Int(value);
        }
        if is_float_ty(own) {
            let exact = match number {
                Number::Int(value) => FloatLit::from_integer(*value),
                Number::Float(value) => value.clone(),
            };
            let exact = match scale {
                Some(scale) => exact.scaled(scale),
                None => exact,
            };
            let is_integer =
                matches!(number, Number::Int(_)) && scale.is_none_or(|scale| scale.fract() == 0.0);
            let value = if own == EvalTy::F32 {
                CVal::F32(exact.to_f32())
            } else {
                CVal::F64(exact.to_f64())
            };
            let finite = match value {
                CVal::F32(value) => value.is_finite(),
                CVal::F64(value) => value.is_finite(),
                _ => false,
            };
            let exactly = !is_integer || exact_integer(&value, number, scale);
            if !finite || !exactly {
                self.error_at_node(
                    Code::DoesNotFit,
                    node,
                    format!("this value does not fit `{}`", ty_name(own)),
                );
                return CVal::Error;
            }
            return value;
        }
        CVal::Error
    }

    fn integer(&mut self, node: NodeId, own: EvalTy, value: Option<i128>) -> CVal {
        let Some(value) = value.filter(|value| *value != i128::MIN) else {
            self.error_at_node(
                Code::IntegerOverflow,
                node,
                "this value's magnitude reaches 2¹²⁷",
            );
            return CVal::Error;
        };
        if let Some(bits) = int_bits(own)
            && !fits(value, bits)
        {
            self.error_at_node(
                Code::ConstantOverflow,
                node,
                format!("this `{}` operation saturates", ty_name(own)),
            );
            return CVal::Error;
        }
        CVal::Int(value)
    }

    fn binary(
        &mut self,
        node: NodeId,
        op: BinaryOp,
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Option<CVal> {
        let Op::Binary(_, lhs, rhs) = self.node(node).op else {
            return Some(CVal::Error);
        };
        match op {
            BinaryOp::And | BinaryOp::Or => {
                let a = self.child(lhs, EvalTy::Bool, ctx, need);
                let b = self.child(rhs, EvalTy::Bool, ctx, need);
                Some(match (a?, b?) {
                    (CVal::Bool(a), CVal::Bool(b)) => {
                        CVal::Bool(if op == BinaryOp::And { a && b } else { a || b })
                    }
                    _ => CVal::Error,
                })
            }
            BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Is
            | BinaryOp::IsNot => {
                let a_sig = self.sig(lhs, need);
                let b_sig = self.sig(rhs, need);
                let (a_sig, b_sig) = (a_sig?, b_sig?);
                let ty = match pair_type(&a_sig, &b_sig) {
                    Pair::Ty(ty) => ty,
                    Pair::FlexibleFloat => {
                        let a = self.child(lhs, EvalTy::F64, ctx, need);
                        let b = self.child(rhs, EvalTy::F64, ctx, need);
                        let fits_f32 = |value: &CVal| match value {
                            CVal::F64(value) => value.is_nan() || (*value as f32).is_finite(),
                            _ => true,
                        };
                        let (a, b) = (a?, b?);
                        if fits_f32(&a) && fits_f32(&b) {
                            EvalTy::F32
                        } else {
                            EvalTy::F64
                        }
                    }
                };
                let a = self.child(lhs, ty, ctx, need);
                let b = self.child(rhs, ty, ctx, need);
                Some(compare(op, &a?, &b?))
            }
            BinaryOp::Add if own == EvalTy::Str => {
                let a = self.child(lhs, EvalTy::Str, ctx, need);
                let b = self.child(rhs, EvalTy::Str, ctx, need);
                Some(match (a?, b?) {
                    (CVal::Str(a), CVal::Str(b)) => CVal::Str(Rc::from(format!("{a}{b}").as_str())),
                    _ => CVal::Error,
                })
            }
            BinaryOp::IntDiv if is_float_ty(own) => {
                let a = self.child(lhs, EvalTy::Default, ctx, need);
                let b = self.child(rhs, EvalTy::Default, ctx, need);
                Some(match (a?, b?) {
                    (CVal::Int(a), CVal::Int(b)) => {
                        let value = self.integer(node, EvalTy::Exact, exact_binary(op, a, b));
                        self.integer_to_float(node, value, own)
                    }
                    _ => CVal::Error,
                })
            }
            BinaryOp::Div => {
                let a = self.float_operand(lhs, own, ctx, need);
                let b = self.float_operand(rhs, own, ctx, need);
                Some(match (a?, b?) {
                    (CVal::F32(a), CVal::F32(b)) => CVal::F32(a / b),
                    (CVal::F64(a), CVal::F64(b)) => CVal::F64(a / b),
                    _ => CVal::Error,
                })
            }
            _ => {
                let a = self.child(lhs, own, ctx, need);
                let b = self.child(rhs, own, ctx, need);
                Some(match (a?, b?) {
                    (CVal::Int(a), CVal::Int(b)) => self.integer(node, own, exact_binary(op, a, b)),
                    (CVal::F32(a), CVal::F32(b)) => CVal::F32(float_binary(op, a, b)),
                    (CVal::F64(a), CVal::F64(b)) => CVal::F64(float_binary(op, a, b)),
                    _ => CVal::Error,
                })
            }
        }
    }

    fn builtin(
        &mut self,
        node: NodeId,
        builtin: Builtin,
        args: &[NodeId],
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Option<CVal> {
        if builtin == Builtin::Rand {
            let values: Vec<Option<CVal>> = args
                .iter()
                .map(|arg| self.child(*arg, EvalTy::I64, ctx, need))
                .collect();
            let values: Vec<CVal> = values.into_iter().collect::<Option<_>>()?;
            let ints: Vec<i64> = values
                .iter()
                .filter_map(|value| match value {
                    CVal::Int(value) => i64::try_from(*value).ok(),
                    _ => None,
                })
                .collect();
            return Some(match ints.as_slice() {
                [seed, n] => CVal::F32(symphonyscript_core::rand(*seed, *n)),
                _ => CVal::Error,
            });
        }
        let values: Vec<Option<CVal>> = if is_float_builtin(builtin) {
            args.iter()
                .map(|arg| self.float_operand(*arg, own, ctx, need))
                .collect()
        } else {
            args.iter()
                .map(|arg| self.child(*arg, own, ctx, need))
                .collect()
        };
        let values: Vec<CVal> = values.into_iter().collect::<Option<_>>()?;
        if let Some(ints) = values
            .iter()
            .map(|value| match value {
                CVal::Int(value) => Some(*value),
                _ => None,
            })
            .collect::<Option<Vec<i128>>>()
        {
            let value = match (builtin, ints.as_slice()) {
                (Builtin::Abs, [x]) => x.checked_abs(),
                (Builtin::Min, [a, b]) => Some(*a.min(b)),
                (Builtin::Max, [a, b]) => Some(*a.max(b)),
                (Builtin::Clamp, [x, lo, hi]) => Some(*x.max(lo).min(hi)),
                _ => return Some(CVal::Error),
            };
            return Some(self.integer(node, own, value));
        }
        let floats32: Option<Vec<f32>> = values
            .iter()
            .map(|value| match value {
                CVal::F32(value) => Some(*value),
                _ => None,
            })
            .collect();
        if let Some(floats) = floats32 {
            return Some(CVal::F32(f32::spec_builtin(builtin, &floats)));
        }
        let floats64: Option<Vec<f64>> = values
            .iter()
            .map(|value| match value {
                CVal::F64(value) => Some(*value),
                _ => None,
            })
            .collect();
        Some(match floats64 {
            Some(floats) => CVal::F64(f64::spec_builtin(builtin, &floats)),
            None => CVal::Error,
        })
    }

    fn concat(&mut self, pieces: &[Piece], ctx: Ctx, need: &mut Vec<Query>) -> Option<CVal> {
        let mut values = Vec::new();
        for piece in pieces {
            if let Piece::Value(node) = piece {
                let sig = self.sig(*node, need);
                let ty = sig.map(|sig| own_type(&sig, EvalTy::Default));
                values.push(ty.and_then(|ty| self.child(*node, ty, ctx, need)));
            }
        }
        if values.iter().any(Option::is_none) {
            return None;
        }
        let mut values = values.into_iter().flatten();
        let mut out = String::new();
        for piece in pieces {
            match piece {
                Piece::Text(text) => out.push_str(text),
                Piece::Value(_) => {
                    let value = values.next().unwrap_or(CVal::Error);
                    let vocabulary = self.vocabulary;
                    let variant = |id, index: u16| {
                        vocabulary
                            .enum_decl(id)
                            .and_then(|decl| decl.variants.get(usize::from(index)).cloned())
                            .unwrap_or_default()
                    };
                    match text(&value, variant) {
                        Some(piece) => out.push_str(&piece),
                        None => return Some(CVal::Error),
                    }
                }
            }
        }
        Some(CVal::Str(Rc::from(out.as_str())))
    }

    pub(in crate::semantics::analysis) fn fit(
        &mut self,
        node: NodeId,
        value: CVal,
        ty: EvalTy,
    ) -> CVal {
        let fits_type = match (&value, ty) {
            (CVal::Int(value), EvalTy::I32) => fits(*value, 32),
            (CVal::Int(value), EvalTy::I64) => fits(*value, 64),
            (CVal::F32(value), EvalTy::F32) => value.is_finite() || value.is_nan(),
            (CVal::F64(value), EvalTy::F64) => value.is_finite() || value.is_nan(),
            _ => true,
        };
        if !fits_type {
            self.error_at_node(
                Code::DoesNotFit,
                node,
                format!("this value does not fit `{}`", ty_name(ty)),
            );
            return CVal::Error;
        }
        value
    }

    fn float_operand(
        &mut self,
        operand: NodeId,
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Option<CVal> {
        let sig = self.sig(operand, need)?;
        if sig.kind != Kind::Int || !is_float_ty(own) {
            return self.child(operand, own, ctx, need);
        }
        let value = self.child(operand, own_type(&sig, EvalTy::Exact), ctx, need)?;
        Some(self.integer_to_float(operand, value, own))
    }

    pub(in crate::semantics::analysis) fn integer_to_float(
        &mut self,
        node: NodeId,
        value: CVal,
        ty: EvalTy,
    ) -> CVal {
        let CVal::Int(integer) = value else {
            return value;
        };
        let converted = convert(value, ty);
        let exactly = match converted {
            CVal::F32(float) => represents(integer, f64::from(float)),
            CVal::F64(float) => represents(integer, float),
            _ => true,
        };
        let flexible = self
            .memo
            .sigs
            .get(&node)
            .is_some_and(|sig| !matches!(sig.width, Width::Fixed(_)));
        if flexible && !exactly {
            self.error_at_node(
                Code::DoesNotFit,
                node,
                format!(
                    "the integer {integer} converts to `{}` inexactly",
                    ty_name(ty)
                ),
            );
            return CVal::Error;
        }
        converted
    }

    fn scaled(&self, node: NodeId, value: CVal) -> CVal {
        if matches!(self.node(node).op, Op::Number(..)) {
            return value;
        }
        let Some(scale) = self.memo.default_scales.get(&node).copied() else {
            return value;
        };
        match value {
            CVal::Int(value) if scale.fract() == 0.0 && scale.abs() < 1e38 => value
                .checked_mul(scale as i128)
                .map_or(CVal::Error, CVal::Int),
            CVal::F32(value) => CVal::F32(value * scale as f32),
            CVal::F64(value) => CVal::F64(value * scale),
            _ => CVal::Error,
        }
    }
}

fn scaled_into(value: CVal, scale: f64, ty: EvalTy) -> CVal {
    let CVal::Int(integer) = value else {
        return CVal::Error;
    };
    let exact = FloatLit::from_integer(integer).scaled(scale);
    match ty {
        EvalTy::F32 => CVal::F32(exact.to_f32()),
        _ => CVal::F64(exact.to_f64()),
    }
}

fn exact_integer(value: &CVal, number: &Number, scale: Option<f64>) -> bool {
    let Number::Int(integer) = number else {
        return true;
    };
    let expected = match scale {
        Some(scale) if scale.fract() == 0.0 && scale.abs() < 1e38 => {
            integer.checked_mul(scale as i128)
        }
        Some(_) => return true,
        None => Some(*integer),
    };
    let Some(expected) = expected else {
        return false;
    };
    match value {
        CVal::F32(value) => represents(expected, f64::from(*value)),
        CVal::F64(value) => represents(expected, *value),
        _ => false,
    }
}

fn float_binary<
    F: SpecFloat
        + std::ops::Add<Output = F>
        + std::ops::Sub<Output = F>
        + std::ops::Mul<Output = F>
        + std::ops::Div<Output = F>,
>(
    op: BinaryOp,
    a: F,
    b: F,
) -> F {
    match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div => a / b,
        BinaryOp::Rem => a.spec_rem(b),
        _ => a.spec_pow(b),
    }
}

fn compare(op: BinaryOp, a: &CVal, b: &CVal) -> CVal {
    let ordering = match (a, b) {
        (CVal::Int(a), CVal::Int(b)) => a.partial_cmp(b),
        (CVal::F32(a), CVal::F32(b)) => a.partial_cmp(b),
        (CVal::F64(a), CVal::F64(b)) => a.partial_cmp(b),
        (CVal::Bool(a), CVal::Bool(b)) => a.partial_cmp(b),
        (CVal::Enum(x, a), CVal::Enum(y, b)) if x == y => a.partial_cmp(b),
        (CVal::Str(a), CVal::Str(b)) => a.partial_cmp(b),
        _ => return CVal::Error,
    };
    let result = match op {
        BinaryOp::Is => ordering == Some(std::cmp::Ordering::Equal),
        BinaryOp::IsNot => ordering != Some(std::cmp::Ordering::Equal),
        BinaryOp::Lt => ordering == Some(std::cmp::Ordering::Less),
        BinaryOp::Le => matches!(
            ordering,
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        BinaryOp::Gt => ordering == Some(std::cmp::Ordering::Greater),
        BinaryOp::Ge => matches!(
            ordering,
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
        _ => return CVal::Error,
    };
    CVal::Bool(result)
}

pub(super) fn convert(value: CVal, target: EvalTy) -> CVal {
    match (value, target) {
        (CVal::Int(value), EvalTy::I32) => {
            CVal::Int(value.clamp(i128::from(i32::MIN), i128::from(i32::MAX)))
        }
        (CVal::Int(value), EvalTy::I64) => {
            CVal::Int(value.clamp(i128::from(i64::MIN), i128::from(i64::MAX)))
        }
        (CVal::Int(value), EvalTy::F32) => CVal::F32(value as f32),
        (CVal::Int(value), EvalTy::F64) => CVal::F64(value as f64),
        (CVal::F32(value), EvalTy::I32) => CVal::Int(i128::from(value as i32)),
        (CVal::F32(value), EvalTy::I64) => CVal::Int(i128::from(value as i64)),
        (CVal::F64(value), EvalTy::I32) => CVal::Int(i128::from(value as i32)),
        (CVal::F64(value), EvalTy::I64) => CVal::Int(i128::from(value as i64)),
        (CVal::F32(value), EvalTy::F64) => CVal::F64(f64::from(value)),
        (CVal::F64(value), EvalTy::F32) => CVal::F32(value as f32),
        (value, _) => value,
    }
}

pub(super) fn convert_to(value: CVal, ty: EvalTy) -> CVal {
    match (&value, ty) {
        (CVal::Int(_), EvalTy::F32 | EvalTy::F64) | (CVal::F32(_), EvalTy::F64) => {
            convert(value, ty)
        }
        (CVal::F32(_) | CVal::F64(_), EvalTy::I32 | EvalTy::I64 | EvalTy::Exact) => CVal::Error,
        _ => value,
    }
}

pub(super) fn ty_name(ty: EvalTy) -> &'static str {
    match ty {
        EvalTy::I32 => "i32",
        EvalTy::I64 => "i64",
        EvalTy::F32 => "f32",
        EvalTy::F64 => "f64",
        EvalTy::Bool => "bool",
        _ => "the type",
    }
}
