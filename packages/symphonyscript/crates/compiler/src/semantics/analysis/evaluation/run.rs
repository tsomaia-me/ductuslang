use std::rc::Rc;

use crate::semantics::builtin::Builtin;
use symphonyscript_core::{BinaryOp, Scalar, UnaryOp};

use super::concat_part::ConcatPart;
use super::ctx::Ctx;
use super::is_f32::is_f32;
use super::is_int_ty::is_int_ty;
use super::operand_post::OperandPost;
use super::own_type::own_type;
use super::ran::Ran;
use super::text::text;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::lengths_of::lengths_of;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::shape_need::ShapeNeed;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::widens::widens;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::{FloatLit, Number, Suffix};
use crate::semantics::operators::convert_to;
use crate::semantics::operators::int_range;
use crate::semantics::operators::is_float_ty;
use crate::semantics::operators::represents;
use crate::semantics::operators::ty_name;
use crate::semantics::operators::{converted_integer, integer};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_eval(
        &mut self,
        node: NodeId,
        ty: EvalTy,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let program = match self.program_for(node, ty)? {
            Ok(program) => program,
            Err(needs) => {
                need.extend(needs);
                return Ok(false);
            }
        };
        Ok(match self.run_program(Ctx::Global, &program, 0, need)? {
            Ran::Value(value) => {
                self.memo.evals.insert((node, ty), value);
                self.memo.programs.remove(&(node, ty));
                true
            }
            Ran::Suspended(_) | Ran::Abandoned | Ran::Ended => false,
        })
    }

    pub(super) fn expected_own(
        &self,
        node: NodeId,
        sig: &Sig,
        ty: EvalTy,
    ) -> Result<EvalTy, Internal> {
        if let Op::Recursive { .. } = self.node(node)?.op
            && sig.kind == Kind::Int
            && expected_scalar(ty).is_some_and(is_integer)
        {
            return Ok(ty);
        }
        let (Width::Fixed(scalar), Some(expected)) = (sig.width, expected_scalar(ty)) else {
            return Ok(own_type(sig, ty));
        };
        Ok(
            if scalar != expected
                && widens(scalar, expected)
                && takes_expected(&self.node(node)?.op)
            {
                ty
            } else {
                own_type(sig, ty)
            },
        )
    }

    pub(super) fn shape_held(
        &mut self,
        node: NodeId,
        value: CVal,
        need: &ShapeNeed,
    ) -> Result<CVal, Internal> {
        if need.by_taken_arm || value.is_error() || lengths_of(&value) == need.lengths {
            return Ok(value);
        }
        match need.at {
            Some((loc, scope)) => self.error(need.code, loc, scope, need.message.clone())?,
            None => self.error_at_node(need.code, node, need.message.clone())?,
        }
        Ok(CVal::Error)
    }

    pub(super) fn takes_a_type(&self, node: NodeId) -> Result<bool, Internal> {
        Ok(match self.node(node)?.op {
            Op::Number(..) | Op::Use(_) => true,
            Op::Unary(UnaryOp::Neg, operand) => matches!(self.node(operand)?.op, Op::Number(..)),
            Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Read(_)
            | Op::Chosen(_)
            | Op::Arg(_)
            | Op::Slot(_)
            | Op::Unary(UnaryOp::Not, _)
            | Op::Binary(..)
            | Op::If(..)
            | Op::Match { .. }
            | Op::Pick { .. }
            | Op::Element { .. }
            | Op::Array(_)
            | Op::Convert(..)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::History { .. }
            | Op::Stepped(_)
            | Op::Native { .. } => false,
        })
    }

    pub(super) fn child(
        &mut self,
        node: NodeId,
        ty: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let Ctx::Frame(frame) = ctx else {
            return Ok(self.cval(Query::Eval(node, ty), need));
        };
        if !self.known_sig(node)?.frame && !self.untaken_unevaluated(frame, node, ty)? {
            return Ok(self.cval(Query::Eval(node, ty), need));
        }
        if let Some(value) = self.frame_value(frame, node, ty)? {
            return Ok(Some(value));
        }
        self.frame_need.push((node, ty));
        Ok(None)
    }

    pub(super) fn literal_value(
        &self,
        node: NodeId,
        number: &Number,
        suffix: Suffix,
        own: EvalTy,
    ) -> Result<Result<CVal, (Code, String)>, Internal> {
        let scale = match suffix {
            Suffix::Unit { family, unit } => Some(self.unit_decl(family, unit)?.scale),
            Suffix::None | Suffix::Type(_) => self
                .memo
                .default_scales
                .get(&node)
                .copied()
                .filter(|scale| is_float_ty(own) || scale.fract() == 0.0),
        };
        if is_int_ty(own) {
            let Number::Int(value) = number else {
                return Ok(Ok(CVal::Error));
            };
            let value = match scale {
                Some(scale) if scale.fract() == 0.0 && scale.abs() < 1e38 => {
                    value.checked_mul(scale as i128)
                }
                Some(_) => return Ok(Ok(CVal::Error)),
                None => Some(*value),
            };
            let Some(value) = value.filter(|value| *value != i128::MIN) else {
                return Ok(Err((
                    Code::IntegerOverflow,
                    "this value's magnitude reaches 2¹²⁷".to_string(),
                )));
            };
            if let Some(range) = int_range(own)
                && !range.contains(&value)
            {
                return Ok(Err((
                    Code::DoesNotFit,
                    format!("the value {value} does not fit `{}`", ty_name(own)),
                )));
            }
            return Ok(Ok(CVal::Int(value)));
        }
        if is_float_ty(own) {
            let exact = match number {
                Number::Int(value) => FloatLit::from_integer(*value),
                Number::Float(value) => value.clone(),
            };
            let exact = match scale {
                Some(scale) => exact.scaled(scale)?,
                None => exact,
            };
            let is_integer =
                matches!(number, Number::Int(_)) && scale.is_none_or(|scale| scale.fract() == 0.0);
            let value = if is_f32(own) {
                CVal::F32(exact.to_f32()?)
            } else {
                CVal::F64(exact.to_f64()?)
            };
            let finite = match value {
                CVal::F32(value) => value.is_finite(),
                CVal::F64(value) => value.is_finite(),
                CVal::Int(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_)
                | CVal::Error => false,
            };
            let exactly = !is_integer || exact_integer(&value, number, scale);
            if !finite || !exactly {
                return Ok(Err((
                    Code::DoesNotFit,
                    format!("this value does not fit `{}`", ty_name(own)),
                )));
            }
            return Ok(Ok(value));
        }
        Ok(Ok(CVal::Error))
    }

    pub(super) fn rand_value(
        &mut self,
        args: &[NodeId],
        values: Vec<CVal>,
    ) -> Result<CVal, Internal> {
        let values: Vec<CVal> = args
            .iter()
            .zip(values)
            .map(|(arg, value)| self.fit(*arg, value, EvalTy::I64))
            .collect::<Result<_, _>>()?;
        let ints: Vec<i64> = values
            .iter()
            .filter_map(|value| match value {
                CVal::Int(value) => i64::try_from(*value).ok(),
                CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_)
                | CVal::Error => None,
            })
            .collect();
        Ok(match ints.as_slice() {
            [seed, n] => CVal::F32(symphonyscript_core::rand(*seed, *n)),
            _ => CVal::Error,
        })
    }

    pub(super) fn concat_value(
        &mut self,
        parts: &[ConcatPart],
        values: Vec<CVal>,
    ) -> Result<CVal, Internal> {
        let mut fitted = Vec::with_capacity(values.len());
        let mut given = values.into_iter();
        for part in parts {
            if let ConcatPart::Value(node, fits) = part {
                let value = given.next().ok_or(Internal::missed("interpolation"))?;
                fitted.push(if *fits {
                    self.fit(*node, value, EvalTy::I64)?
                } else {
                    value
                });
            }
        }
        let mut values = fitted.into_iter();
        let mut out = String::new();
        for part in parts {
            match part {
                ConcatPart::Text(text) => out.push_str(text),
                ConcatPart::Value(..) => {
                    let value = values.next().ok_or(Internal::missed("interpolation"))?;
                    let variant = |id, index: u16| {
                        self.enum_decl(id)?
                            .variants
                            .get(usize::from(index))
                            .cloned()
                            .ok_or(Internal::missed("enum variant"))
                    };
                    match text(&value, variant)? {
                        Some(piece) => out.push_str(&piece),
                        None => return Ok(CVal::Error),
                    }
                }
            }
        }
        Ok(CVal::Str(Rc::from(out.as_str())))
    }

    pub(super) fn operand_post(
        &mut self,
        post: OperandPost,
        value: CVal,
    ) -> Result<CVal, Internal> {
        match post {
            OperandPost::Keep => Ok(value),
            OperandPost::FitThenFloat(operand, ty, own) => {
                let value = self.fit(operand, value, ty)?;
                self.convert_integer(operand, false, value, own)
            }
            OperandPost::Float(operand, unfixed, own) => {
                self.convert_integer(operand, unfixed, value, own)
            }
        }
    }

    pub(in crate::semantics::analysis) fn fit(
        &mut self,
        node: NodeId,
        value: CVal,
        ty: EvalTy,
    ) -> Result<CVal, Internal> {
        if !matches!(value, CVal::Array(_)) {
            crate::semantics::counted_walk::CountedWalk::RunFit.step();
            if fits_in(&value, ty) {
                return Ok(value);
            }
            self.error_at_node(
                Code::DoesNotFit,
                node,
                format!("this value does not fit `{}`", ty_name(ty)),
            )?;
            return Ok(CVal::Error);
        }
        let mut unfit = Vec::new();
        let mut parts = vec![(node, value.clone())];
        while let Some((at, part)) = parts.pop() {
            crate::semantics::counted_walk::CountedWalk::RunFit.step();
            match (&self.node(at)?.op, &part) {
                (Op::Array(elements), CVal::Array(array)) if elements.len() == array.len() => {
                    parts.extend(elements.iter().copied().zip(array.items().iter().cloned()));
                }
                (_, CVal::Array(array)) => {
                    if !array.all_leaves(|leaf| fits_in(leaf, ty)) {
                        unfit.push(at);
                    }
                }
                (_, leaf) => {
                    if !fits_in(leaf, ty) {
                        unfit.push(at);
                    }
                }
            }
        }
        if unfit.is_empty() {
            return Ok(value);
        }
        unfit.sort();
        for at in unfit {
            self.error_at_node(
                Code::DoesNotFit,
                at,
                format!("this value does not fit `{}`", ty_name(ty)),
            )?;
        }
        Ok(CVal::Error)
    }

    pub(super) fn typed_child(
        &mut self,
        node: NodeId,
        (own, flows): (EvalTy, bool),
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let sig = some!(self.sig(node, need));
        let request =
            if !flows && expected_scalar(own).is_some() && matches!(sig.width, Width::Fixed(_)) {
                EvalTy::Default
            } else {
                own
            };
        if request == own || !sig.is_numeric() {
            return self.child(node, own, ctx, need);
        }
        let value = some!(self.child(node, request, ctx, need)?);
        Ok(Some(map_leaves(value, &mut FastMap::default(), |leaf| {
            Ok(convert_to(leaf, own))
        })?))
    }

    pub(super) fn scaled_leaf(
        &mut self,
        node: NodeId,
        scale: f64,
        value: CVal,
        target: EvalTy,
    ) -> Result<CVal, Internal> {
        Ok(match value {
            CVal::Int(value) if scale.fract() == 0.0 && scale.abs() < 1e38 => {
                self.integer(node, EvalTy::Exact, value.checked_mul(scale as i128))?
            }
            CVal::F32(value) => scaled_float(f64::from(value), scale, target)?,
            CVal::F64(value) => scaled_float(value, scale, target)?,
            CVal::Int(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_)
            | CVal::Array(_)
            | CVal::Error => CVal::Error,
        })
    }

    pub(super) fn integer(
        &mut self,
        node: NodeId,
        own: EvalTy,
        value: Option<i128>,
    ) -> Result<CVal, Internal> {
        integer(self, node, own, value)
    }

    pub(super) fn convert_integer(
        &mut self,
        node: NodeId,
        flexible: bool,
        value: CVal,
        ty: EvalTy,
    ) -> Result<CVal, Internal> {
        converted_integer(self, node, flexible, value, ty)
    }
}

fn fits_in(value: &CVal, ty: EvalTy) -> bool {
    match (value, ty) {
        (CVal::Int(value), EvalTy::U8 | EvalTy::I32 | EvalTy::I64) => {
            int_range(ty).is_some_and(|range| range.contains(value))
        }
        (CVal::F32(value), EvalTy::F32 | EvalTy::DefaultF32) => value.is_finite(),
        (CVal::F64(value), EvalTy::F64 | EvalTy::DefaultF64) => value.is_finite(),
        _ => true,
    }
}

pub(super) fn scaled_into(value: CVal, scale: f64, ty: EvalTy) -> Result<CVal, Internal> {
    let CVal::Int(integer) = value else {
        return Ok(CVal::Error);
    };
    let exact = FloatLit::from_integer(integer).scaled(scale)?;
    Ok(if is_f32(ty) {
        CVal::F32(exact.to_f32()?)
    } else {
        CVal::F64(exact.to_f64()?)
    })
}

fn scaled_float(value: f64, scale: f64, target: EvalTy) -> Result<CVal, Internal> {
    if !value.is_finite() {
        return Ok(if is_f32(target) {
            CVal::F32((value * scale) as f32)
        } else {
            CVal::F64(value * scale)
        });
    }
    let magnitude = FloatLit::from_integer(1)
        .scaled(value.abs())?
        .scaled(scale)?;
    let exact = if value.is_sign_negative() {
        magnitude.negated()
    } else {
        magnitude
    };
    Ok(if is_f32(target) {
        CVal::F32(exact.to_f32()?)
    } else {
        CVal::F64(exact.to_f64()?)
    })
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
        CVal::Int(_)
        | CVal::Bool(_)
        | CVal::Enum(..)
        | CVal::Str(_)
        | CVal::Node(_)
        | CVal::Array(_)
        | CVal::Error => false,
    }
}

pub(super) fn expected_scalar(ty: EvalTy) -> Option<Scalar> {
    match ty {
        EvalTy::U8 => Some(Scalar::U8),
        EvalTy::I32 => Some(Scalar::I32),
        EvalTy::I64 => Some(Scalar::I64),
        EvalTy::F32 => Some(Scalar::F32),
        EvalTy::F64 => Some(Scalar::F64),
        EvalTy::Exact
        | EvalTy::Default
        | EvalTy::DefaultF32
        | EvalTy::DefaultF64
        | EvalTy::Bool
        | EvalTy::Enum(_)
        | EvalTy::Str
        | EvalTy::Node => None,
    }
}

fn takes_expected(op: &Op) -> bool {
    match op {
        Op::Unary(UnaryOp::Neg, _)
        | Op::Use(_)
        | Op::Chosen(_)
        | Op::If(..)
        | Op::Match { .. }
        | Op::Pick { .. }
        | Op::Element { .. }
        | Op::Array(_)
        | Op::Join(..)
        | Op::Slice { .. } => true,
        Op::Binary(op, ..) => matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::IntDiv
                | BinaryOp::Rem
                | BinaryOp::Pow
        ),
        Op::Builtin(builtin, _) => !matches!(
            builtin,
            Builtin::Rand | Builtin::Len | Builtin::Capacity | Builtin::Bytes
        ),
        Op::Number(..)
        | Op::Const(_)
        | Op::Bool(_)
        | Op::Str(_)
        | Op::Concat(_)
        | Op::Variant(..)
        | Op::Bare(_)
        | Op::Pi
        | Op::NodeRef(_)
        | Op::Read(_)
        | Op::Arg(_)
        | Op::Slot(_)
        | Op::Unary(UnaryOp::Not, _)
        | Op::Convert(..)
        | Op::Annotate(..)
        | Op::Recursive { .. }
        | Op::For(_)
        | Op::ToRead { .. }
        | Op::History { .. }
        | Op::Stepped(_)
        | Op::Native { .. } => false,
    }
}

pub(super) const SINGLE_OPERANDS: &str =
    "only `+` takes arrays, which it joins; every other operator takes single values";

pub(super) fn narrowed(value: CVal, own: EvalTy) -> Result<CVal, Internal> {
    map_leaves(value, &mut FastMap::default(), |leaf| {
        Ok::<CVal, Internal>(convert_to(leaf, own))
    })
}

pub(super) fn fits_f32(value: &CVal) -> bool {
    match value {
        CVal::F64(value) => value.is_nan() || (*value as f32).is_finite(),
        CVal::Int(_)
        | CVal::F32(_)
        | CVal::Bool(_)
        | CVal::Enum(..)
        | CVal::Str(_)
        | CVal::Node(_)
        | CVal::Array(_)
        | CVal::Error => true,
    }
}

pub(super) fn wide_fits_f32(value: &CVal) -> bool {
    let within = |leaf: &CVal| match leaf {
        CVal::F64(value) => value.is_nan() || (*value as f32).is_finite(),
        CVal::Int(_)
        | CVal::F32(_)
        | CVal::Bool(_)
        | CVal::Enum(..)
        | CVal::Str(_)
        | CVal::Node(_)
        | CVal::Array(_)
        | CVal::Error => false,
    };
    match value {
        CVal::Array(array) => array.all_leaves(within),
        leaf => within(leaf),
    }
}
