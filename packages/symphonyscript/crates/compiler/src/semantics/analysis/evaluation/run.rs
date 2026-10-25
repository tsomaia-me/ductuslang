use std::rc::Rc;

use symphonyscript_core::{BinaryOp, Builtin, SpecFloat, UnaryOp, Value};

use super::convert::convert;
use super::convert_to::convert_to;
use super::ctx::Ctx;
use super::exact_binary::exact_binary;
use super::int_range::int_range;
use super::is_chosen_float::is_chosen_float;
use super::is_f32::is_f32;
use super::is_float_builtin::is_float_builtin;
use super::is_float_ty::is_float_ty;
use super::is_int_ty::is_int_ty;
use super::of_scalar::of_scalar;
use super::own_type::own_type;
use super::pair::Pair;
use super::pair_type::pair_type;
use super::represents::represents;
use super::text::text;
use super::ty_name::ty_name;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_array::CArray;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::is_integer::is_integer;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::lengths_of::lengths_of;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::piece::Piece;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::{FloatLit, Number, Suffix};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_eval(
        &mut self,
        node: NodeId,
        ty: EvalTy,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        Ok(match self.evaluate(node, ty, Ctx::Global, need)? {
            Some(value) => {
                self.memo.evals.insert((node, ty), value);
                true
            }
            None => false,
        })
    }

    pub(super) fn evaluate(
        &mut self,
        node: NodeId,
        ty: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let sig = some!(self.sig(node, need));
        if sig.is_error() || sig.reactive {
            return Ok(Some(CVal::Error));
        }
        let own = own_type(&sig, ty);
        if let Width::Widened(integer) = sig.width
            && let Some(scale) = self.memo.default_scales.get(&node).copied()
        {
            let value = some!(self.compute(node, of_scalar(integer), ctx, need)?);
            let value = map_leaves(value, |leaf| {
                Ok(convert_to(scaled_into(leaf, scale, own)?, ty))
            })?;
            return Ok(Some(self.shape_checked(node, value)?));
        }
        let value = if own == EvalTy::Default && sig.kind == Kind::Float {
            let wide = some!(self.child(node, EvalTy::DefaultF64, ctx, need)?);
            let leaves = match &wide {
                CVal::Array(array) => array.leaves(),
                leaf => vec![leaf.clone()],
            };
            let outside_f32 = leaves.iter().any(|leaf| {
                matches!(leaf, CVal::F64(value) if !value.is_nan() && !(*value as f32).is_finite())
            });
            let all_f64 = leaves.iter().all(|leaf| matches!(leaf, CVal::F64(_)));
            if all_f64 && !outside_f32 {
                some!(self.child(node, EvalTy::DefaultF32, ctx, need)?)
            } else {
                wide
            }
        } else {
            let value = some!(self.compute(node, own, ctx, need)?);
            if matches!(self.node(node)?.op, Op::Use(_))
                && self.memo.default_scales.contains_key(&node)
            {
                self.memo
                    .before_unit_reading
                    .insert((node, ty), value.clone());
            }
            let target = if is_float_ty(ty) { ty } else { own };
            self.scaled(node, value, target)?
        };
        if sig.kind == Kind::Int
            && sig.width == Width::Flexible
            && int_range(ty).is_some()
            && self.takes_a_type(node)?
        {
            let value = self.shape_checked(node, value)?;
            return Ok(Some(self.fit(node, value, ty)?));
        }
        let value = map_leaves(value, |leaf| Ok(convert_to(leaf, ty)))?;
        Ok(Some(self.shape_checked(node, value)?))
    }

    fn shape_checked(&mut self, node: NodeId, value: CVal) -> Result<CVal, Internal> {
        let Some(need) = self.memo.shapes.get(&node).cloned() else {
            return Ok(value);
        };
        if value.is_error() || lengths_of(&value) == need.lengths {
            return Ok(value);
        }
        self.error_at_node(need.code, node, need.message)?;
        Ok(CVal::Error)
    }

    fn element(
        &mut self,
        base: NodeId,
        index: NodeId,
        value: CVal,
        position: i128,
    ) -> Result<CVal, Internal> {
        let array = match value {
            CVal::Array(array) => array,
            CVal::Error => return Ok(CVal::Error),
            CVal::Int(_)
            | CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_) => {
                self.error_at_node(Code::NotAnArray, base, "only an array is indexed")?;
                return Ok(CVal::Error);
            }
        };
        let item = usize::try_from(position)
            .ok()
            .and_then(|position| array.items().get(position).cloned());
        Ok(match item {
            Some(item) => item,
            None => {
                self.error_at_node(
                    Code::IndexOutOfRange,
                    index,
                    format!(
                        "the index {position} is outside the array of length {}",
                        array.items().len()
                    ),
                )?;
                CVal::Error
            }
        })
    }

    fn takes_a_type(&self, node: NodeId) -> Result<bool, Internal> {
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
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::History { .. }
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
        if !self.known_sig(node)?.frame {
            return Ok(self.cval(Query::Eval(node, ty), need));
        }
        if let Some(value) = self.frame_value(frame, node, ty)? {
            return Ok(Some(value));
        }
        self.frame_need.push((node, ty));
        Ok(None)
    }

    fn compute(
        &mut self,
        node: NodeId,
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let op = self.node(node)?.op.clone();
        Ok(Some(match op {
            Op::Read(_) | Op::History { .. } => CVal::Error,
            Op::Join(lhs, rhs) => return self.join((lhs, rhs), own, ctx, need),
            Op::Slice { base, lower, upper } => {
                return self.sliced(base, (lower, upper), own, ctx, need);
            }
            Op::Native { native, args } => return self.native(native, &args, ctx, need),
            Op::Builtin(Builtin::Len, args) => {
                let operand = *args.first().ok_or(Internal::missed("len operand"))?;
                return self.length(operand, ctx, need);
            }
            Op::Builtin(Builtin::Bytes, args) => {
                let operand = *args.first().ok_or(Internal::missed("bytes operand"))?;
                return self.bytes(operand, ctx, need);
            }
            Op::Pick { index, items } => {
                let index_sig = some!(self.sig(index, need));
                let index_ty = own_type(&index_sig, EvalTy::Exact);
                let position = some!(self.child(index, index_ty, ctx, need)?);
                let Some(position) = int_position(position) else {
                    return Ok(Some(CVal::Error));
                };
                let item = usize::try_from(position)
                    .ok()
                    .and_then(|position| items.get(position).copied());
                match item {
                    Some(item) => some!(self.child(item, own, ctx, need)?),
                    None => {
                        self.error_at_node(
                            Code::IndexOutOfRange,
                            index,
                            format!(
                                "the index {position} is outside the array of length {}",
                                items.len()
                            ),
                        )?;
                        CVal::Error
                    }
                }
            }
            Op::Element { base, index } => {
                let value = self.child(base, own, ctx, need)?;
                let index_sig = some!(self.sig(index, need));
                let index_ty = own_type(&index_sig, EvalTy::Exact);
                let position = self.child(index, index_ty, ctx, need)?;
                let (value, position) = (some!(value), some!(position));
                let Some(position) = int_position(position) else {
                    return Ok(Some(CVal::Error));
                };
                self.element(base, index, value, position)?
            }
            Op::Array(elements) => {
                let values: Vec<Option<CVal>> = elements
                    .iter()
                    .map(|element| self.child(*element, own, ctx, need))
                    .collect::<Result<_, _>>()?;
                let values: Vec<CVal> = some!(values.into_iter().collect::<Option<_>>());
                if values.iter().any(CVal::is_error) {
                    return Ok(Some(CVal::Error));
                }
                let Some(first) = values.first() else {
                    return Ok(Some(CVal::Array(CArray::new(Vec::new()))));
                };
                let inner = lengths_of(first);
                if values.iter().any(|value| lengths_of(value) != inner) {
                    self.error_at_node(
                        Code::RaggedArray,
                        node,
                        "the elements of an array are arrays of one length, or all single values",
                    )?;
                    return Ok(Some(CVal::Error));
                }
                CVal::Array(CArray::new(values))
            }
            Op::Arg(slot) => match ctx {
                Ctx::Frame(frame) => self.frame_arg(frame, slot)?,
                Ctx::Global => return Err(Internal::missed("evaluation frame")),
            },
            Op::Number(number, suffix) => self.literal(node, &number, suffix, own)?,
            Op::Const(value) => match value {
                Value::U8(value) => CVal::Int(i128::from(value)),
                Value::I32(value) => CVal::Int(i128::from(value)),
                Value::I64(value) => CVal::Int(i128::from(value)),
                Value::F32(value) => CVal::F32(value),
                Value::F64(value) => CVal::F64(value),
                Value::Bool(value) => CVal::Bool(value),
                Value::Enum(id, index) => CVal::Enum(id, index),
                Value::Str(_) => CVal::Error,
            },
            Op::Use(target) | Op::Chosen(target) => some!(self.child(target, own, ctx, need)?),
            Op::Bool(value) => CVal::Bool(value),
            Op::Str(text) => CVal::Str(Rc::from(text.as_str())),
            Op::Variant(id, index) => CVal::Enum(id, index),
            Op::NodeRef(item) => CVal::Node(item),
            Op::Pi => {
                if is_f32(own) {
                    CVal::F32(std::f32::consts::PI)
                } else {
                    CVal::F64(std::f64::consts::PI)
                }
            }
            Op::Bare(name) => match own {
                EvalTy::Enum(id) => match self.variants.position(id, &name)? {
                    Some(index) => CVal::Enum(id, index),
                    None => {
                        let enum_name = self.enum_decl(id)?.name.clone();
                        self.error_at_node(
                            Code::UnknownName,
                            node,
                            format!("`{name}` is not a variant of `{enum_name}`"),
                        )?;
                        CVal::Error
                    }
                },
                EvalTy::Exact
                | EvalTy::Default
                | EvalTy::U8
                | EvalTy::I32
                | EvalTy::I64
                | EvalTy::F32
                | EvalTy::F64
                | EvalTy::DefaultF32
                | EvalTy::DefaultF64
                | EvalTy::Bool
                | EvalTy::Str
                | EvalTy::Node => match self.memo.bare_enums.get(&node) {
                    Some(id) => match self.variants.position(*id, &name)? {
                        Some(index) => CVal::Enum(*id, index),
                        None => CVal::Error,
                    },
                    None => CVal::Error,
                },
            },
            Op::Concat(pieces) => return self.concat(&pieces, ctx, need),
            Op::Unary(UnaryOp::Not, operand) => {
                match some!(self.child(operand, EvalTy::Bool, ctx, need)?) {
                    CVal::Bool(value) => CVal::Bool(!value),
                    CVal::Int(_)
                    | CVal::F32(_)
                    | CVal::F64(_)
                    | CVal::Enum(..)
                    | CVal::Str(_)
                    | CVal::Node(_)
                    | CVal::Array(_)
                    | CVal::Error => CVal::Error,
                }
            }
            Op::Unary(UnaryOp::Neg, operand) => {
                let value = some!(self.child(operand, own, ctx, need)?);
                match value {
                    CVal::Int(value) => self.integer(node, own, value.checked_neg())?,
                    CVal::F32(value) => CVal::F32(-value),
                    CVal::F64(value) => CVal::F64(-value),
                    CVal::Bool(_)
                    | CVal::Enum(..)
                    | CVal::Str(_)
                    | CVal::Node(_)
                    | CVal::Array(_)
                    | CVal::Error => CVal::Error,
                }
            }
            Op::Binary(op, lhs, rhs) => return self.binary(node, (op, lhs, rhs), own, ctx, need),
            Op::If(condition, then, otherwise) => {
                let chosen = match some!(self.child(condition, EvalTy::Bool, ctx, need)?) {
                    CVal::Bool(true) => then,
                    CVal::Bool(false) => otherwise,
                    CVal::Int(_)
                    | CVal::F32(_)
                    | CVal::F64(_)
                    | CVal::Enum(..)
                    | CVal::Str(_)
                    | CVal::Node(_)
                    | CVal::Array(_)
                    | CVal::Error => return Ok(Some(CVal::Error)),
                };
                some!(self.child(chosen, own, ctx, need)?)
            }
            Op::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let scrutinee_sig = some!(self.sig(scrutinee, need));
                let Some(scrutinee_ty) = self.scrutinee_ty(scrutinee, &scrutinee_sig) else {
                    return Ok(Some(CVal::Error));
                };
                let value = some!(self.child(scrutinee, scrutinee_ty, ctx, need)?);
                let value = self.default_fit(scrutinee, value)?;
                if value.is_error() {
                    return Ok(Some(CVal::Error));
                }
                let mut chosen = otherwise;
                for (label, arm) in &arms {
                    let label_value = some!(self.child(*label, scrutinee_ty, ctx, need)?);
                    let label_value = self.default_fit(*label, label_value)?;
                    if label_value == value {
                        chosen = Some(*arm);
                        break;
                    }
                }
                match chosen {
                    Some(arm) => some!(self.child(arm, own, ctx, need)?),
                    None => CVal::Error,
                }
            }
            Op::Convert(_, operand) => {
                let operand_ty = if is_float_ty(own) {
                    own
                } else {
                    EvalTy::Default
                };
                let value = some!(self.child(operand, operand_ty, ctx, need)?);
                let value = if is_int_ty(own) {
                    self.default_fit(operand, value)?
                } else {
                    value
                };
                convert(value, own)
            }
            Op::Annotate(_, operand) => {
                let value = some!(self.child(operand, own, ctx, need)?);
                if some!(self.sig(operand, need)).width == Width::Flexible {
                    self.fit(node, value, own)?
                } else {
                    value
                }
            }
            Op::Builtin(builtin, args) => {
                return self.builtin(node, builtin, &args, own, ctx, need);
            }
            Op::Recursive { function, args } => {
                return self.recursive(node, (function, args), own, ctx, need);
            }
            Op::ToRead { wire, attrs, .. } => {
                let destination = some!(self.destination(Query::Destination(wire), need));
                let Destination::Fixed(_) = destination else {
                    return Ok(Some(CVal::Error));
                };
                let (item, attr) = attrs.first().ok_or(Internal::missed("attribute"))?;
                match some!(self.bound(Query::Attr(*item, *attr), need)) {
                    BoundVal::Scalar(Bound::Value(value)) => {
                        some!(self.child(value, own, ctx, need)?)
                    }
                    BoundVal::Scalar(Bound::Cell(_)) | BoundVal::Array(_) | BoundVal::Error => {
                        CVal::Error
                    }
                }
            }
        }))
    }

    fn literal(
        &mut self,
        node: NodeId,
        number: &Number,
        suffix: Suffix,
        own: EvalTy,
    ) -> Result<CVal, Internal> {
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
                return Ok(CVal::Error);
            };
            let value = match scale {
                Some(scale) if scale.fract() == 0.0 && scale.abs() < 1e38 => {
                    value.checked_mul(scale as i128)
                }
                Some(_) => return Ok(CVal::Error),
                None => Some(*value),
            };
            let Some(value) = value.filter(|value| *value != i128::MIN) else {
                self.error_at_node(
                    Code::IntegerOverflow,
                    node,
                    "this value's magnitude reaches 2¹²⁷",
                )?;
                return Ok(CVal::Error);
            };
            if let Some(range) = int_range(own)
                && !range.contains(&value)
            {
                self.error_at_node(
                    Code::DoesNotFit,
                    node,
                    format!("the value {value} does not fit `{}`", ty_name(own)),
                )?;
                return Ok(CVal::Error);
            }
            return Ok(CVal::Int(value));
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
                self.error_at_node(
                    Code::DoesNotFit,
                    node,
                    format!("this value does not fit `{}`", ty_name(own)),
                )?;
                return Ok(CVal::Error);
            }
            return Ok(value);
        }
        Ok(CVal::Error)
    }

    fn integer(
        &mut self,
        node: NodeId,
        own: EvalTy,
        value: Option<i128>,
    ) -> Result<CVal, Internal> {
        let Some(value) = value.filter(|value| *value != i128::MIN) else {
            self.error_at_node(
                Code::IntegerOverflow,
                node,
                "this value's magnitude reaches 2¹²⁷",
            )?;
            return Ok(CVal::Error);
        };
        if let Some(range) = int_range(own)
            && !range.contains(&value)
        {
            self.error_at_node(
                Code::ConstantOverflow,
                node,
                format!("this `{}` operation saturates", ty_name(own)),
            )?;
            return Ok(CVal::Error);
        }
        Ok(CVal::Int(value))
    }

    fn binary(
        &mut self,
        node: NodeId,
        (op, lhs, rhs): (BinaryOp, NodeId, NodeId),
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        Ok(match op {
            BinaryOp::And | BinaryOp::Or => {
                let a = self.child(lhs, EvalTy::Bool, ctx, need)?;
                let b = self.child(rhs, EvalTy::Bool, ctx, need)?;
                Some(match (some!(a), some!(b)) {
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
                let (a_sig, b_sig) = (some!(a_sig), some!(b_sig));
                if matches!(op, BinaryOp::Is | BinaryOp::IsNot)
                    && self.crossed_enums(node, lhs, rhs)?
                {
                    return Ok(Some(CVal::Error));
                }
                let carried = match (a_sig.kind, b_sig.kind) {
                    (Kind::Bare, Kind::Bare) => {
                        self.carried_enum_of(lhs)?.or(self.carried_enum_of(rhs)?)
                    }
                    _ => None,
                };
                let ty = match carried.map_or_else(
                    || pair_type(&a_sig, &b_sig),
                    |id| Pair::Ty(EvalTy::Enum(id)),
                ) {
                    Pair::Ty(ty) => ty,
                    Pair::FlexibleFloat => {
                        let a = self.child(lhs, EvalTy::DefaultF64, ctx, need)?;
                        let b = self.child(rhs, EvalTy::DefaultF64, ctx, need)?;
                        let fits_f32 = |value: &CVal| match value {
                            CVal::F64(value) => value.is_nan() || (*value as f32).is_finite(),
                            CVal::Int(_)
                            | CVal::F32(_)
                            | CVal::Bool(_)
                            | CVal::Enum(..)
                            | CVal::Str(_)
                            | CVal::Node(_)
                            | CVal::Array(_)
                            | CVal::Error => true,
                        };
                        let (a, b) = (some!(a), some!(b));
                        if fits_f32(&a) && fits_f32(&b) {
                            EvalTy::DefaultF32
                        } else {
                            EvalTy::DefaultF64
                        }
                    }
                };
                let a = self.child(lhs, ty, ctx, need)?;
                let b = self.child(rhs, ty, ctx, need)?;
                let (a, b) = (some!(a), some!(b));
                let a = self.operand_fit(lhs, &a_sig, a, ty)?;
                let b = self.operand_fit(rhs, &b_sig, b, ty)?;
                if a.is_error() || b.is_error() {
                    return Ok(Some(CVal::Error));
                }
                Some(compare(op, &a, &b))
            }
            BinaryOp::Add if own == EvalTy::Str => {
                let a = self.child(lhs, EvalTy::Str, ctx, need)?;
                let b = self.child(rhs, EvalTy::Str, ctx, need)?;
                Some(match (some!(a), some!(b)) {
                    (CVal::Str(a), CVal::Str(b)) => CVal::Str(Rc::from(format!("{a}{b}").as_str())),
                    _ => CVal::Error,
                })
            }
            BinaryOp::IntDiv if is_float_ty(own) => {
                let a = self.child(lhs, EvalTy::Default, ctx, need)?;
                let b = self.child(rhs, EvalTy::Default, ctx, need)?;
                Some(match (some!(a), some!(b)) {
                    (CVal::Int(a), CVal::Int(b)) => {
                        let value = self.integer(node, EvalTy::Exact, exact_binary(op, a, b))?;
                        self.integer_to_float(node, value, own)?
                    }
                    _ => CVal::Error,
                })
            }
            BinaryOp::Div => {
                let a = self.float_operand(lhs, &[rhs], own, ctx, need)?;
                let b = self.float_operand(rhs, &[lhs], own, ctx, need)?;
                Some(match (some!(a), some!(b)) {
                    (CVal::F32(a), CVal::F32(b)) => CVal::F32(a / b),
                    (CVal::F64(a), CVal::F64(b)) => CVal::F64(a / b),
                    _ => CVal::Error,
                })
            }
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::IntDiv
            | BinaryOp::Rem
            | BinaryOp::Pow => {
                let a = self.child(lhs, own, ctx, need)?;
                let b = self.child(rhs, own, ctx, need)?;
                Some(match (some!(a), some!(b)) {
                    (CVal::Int(a), CVal::Int(b)) => {
                        self.integer(node, own, exact_binary(op, a, b))?
                    }
                    (CVal::F32(a), CVal::F32(b)) => {
                        CVal::F32(float_binary(op, a, b).ok_or(Internal::missed("float operator"))?)
                    }
                    (CVal::F64(a), CVal::F64(b)) => {
                        CVal::F64(float_binary(op, a, b).ok_or(Internal::missed("float operator"))?)
                    }
                    _ => CVal::Error,
                })
            }
        })
    }

    fn builtin(
        &mut self,
        node: NodeId,
        builtin: Builtin,
        args: &[NodeId],
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        if builtin == Builtin::Rand {
            let values: Vec<Option<CVal>> = args
                .iter()
                .map(|arg| self.child(*arg, EvalTy::I64, ctx, need))
                .collect::<Result<_, _>>()?;
            let values: Vec<CVal> = some!(values.into_iter().collect::<Option<_>>());
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
            return Ok(Some(match ints.as_slice() {
                [seed, n] => CVal::F32(symphonyscript_core::rand(*seed, *n)),
                _ => CVal::Error,
            }));
        }
        let values: Vec<Option<CVal>> = if is_float_builtin(builtin) {
            args.iter()
                .map(|arg| self.float_operand(*arg, args, own, ctx, need))
                .collect::<Result<_, _>>()?
        } else {
            args.iter()
                .map(|arg| self.child(*arg, own, ctx, need))
                .collect::<Result<_, _>>()?
        };
        let values: Vec<CVal> = some!(values.into_iter().collect::<Option<_>>());
        if let Some(ints) = values
            .iter()
            .map(|value| match value {
                CVal::Int(value) => Some(*value),
                CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_)
                | CVal::Error => None,
            })
            .collect::<Option<Vec<i128>>>()
        {
            let value = match (builtin, ints.as_slice()) {
                (Builtin::Abs, [x]) => x.checked_abs(),
                (Builtin::Min, [a, b]) => Some(*a.min(b)),
                (Builtin::Max, [a, b]) => Some(*a.max(b)),
                (Builtin::Clamp, [x, lo, hi]) => Some(*x.max(lo).min(hi)),
                _ => return Ok(Some(CVal::Error)),
            };
            return Ok(Some(self.integer(node, own, value)?));
        }
        let floats32: Option<Vec<f32>> = values
            .iter()
            .map(|value| match value {
                CVal::F32(value) => Some(*value),
                CVal::Int(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_)
                | CVal::Error => None,
            })
            .collect();
        if let Some(floats) = floats32 {
            return Ok(Some(CVal::F32(
                f32::spec_builtin(builtin, &floats).ok_or(Internal::missed("built-in"))?,
            )));
        }
        let floats64: Option<Vec<f64>> = values
            .iter()
            .map(|value| match value {
                CVal::F64(value) => Some(*value),
                CVal::Int(_)
                | CVal::F32(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_)
                | CVal::Error => None,
            })
            .collect();
        Ok(Some(match floats64 {
            Some(floats) => {
                CVal::F64(f64::spec_builtin(builtin, &floats).ok_or(Internal::missed("built-in"))?)
            }
            None => CVal::Error,
        }))
    }

    fn concat(
        &mut self,
        pieces: &[Piece],
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let mut values = Vec::new();
        for piece in pieces {
            if let Piece::Value(node) = piece {
                let sig = self.sig(*node, need);
                let ty = sig.map(|sig| own_type(&sig, EvalTy::Default));
                values.push(match ty {
                    Some(ty) => self.child(*node, ty, ctx, need)?,
                    None => None,
                });
            }
        }
        if values.iter().any(Option::is_none) {
            return Ok(None);
        }
        let nodes = pieces.iter().filter_map(|piece| match piece {
            Piece::Value(node) => Some(*node),
            Piece::Text(_) => None,
        });
        let fitted: Vec<CVal> = nodes
            .zip(values.into_iter().flatten())
            .map(|(node, value)| self.default_fit(node, value))
            .collect::<Result<_, _>>()?;
        let mut values = fitted.into_iter();
        let mut out = String::new();
        for piece in pieces {
            match piece {
                Piece::Text(text) => out.push_str(text),
                Piece::Value(_) => {
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
                        None => return Ok(Some(CVal::Error)),
                    }
                }
            }
        }
        Ok(Some(CVal::Str(Rc::from(out.as_str()))))
    }

    pub(in crate::semantics::analysis) fn fit(
        &mut self,
        node: NodeId,
        value: CVal,
        ty: EvalTy,
    ) -> Result<CVal, Internal> {
        let fits_type = match &value {
            CVal::Array(array) => array.leaves().iter().all(|leaf| fits_in(leaf, ty)),
            leaf => fits_in(leaf, ty),
        };
        if !fits_type {
            self.error_at_node(
                Code::DoesNotFit,
                node,
                format!("this value does not fit `{}`", ty_name(ty)),
            )?;
            return Ok(CVal::Error);
        }
        Ok(value)
    }

    fn operand_fit(
        &mut self,
        node: NodeId,
        sig: &Sig,
        value: CVal,
        ty: EvalTy,
    ) -> Result<CVal, Internal> {
        if sig.width != Width::Flexible || !sig.is_numeric() {
            return Ok(value);
        }
        match ty {
            EvalTy::Exact => self.fit(node, value, EvalTy::I64),
            EvalTy::Default
            | EvalTy::U8
            | EvalTy::I32
            | EvalTy::I64
            | EvalTy::F32
            | EvalTy::F64
            | EvalTy::DefaultF32
            | EvalTy::DefaultF64
            | EvalTy::Bool
            | EvalTy::Enum(_)
            | EvalTy::Str
            | EvalTy::Node => self.fit(node, value, ty),
        }
    }

    pub(in crate::semantics::analysis) fn default_fit(
        &mut self,
        node: NodeId,
        value: CVal,
    ) -> Result<CVal, Internal> {
        let sig = self.known_sig(node)?;
        let flexible_integer = sig.kind == Kind::Int && sig.width == Width::Flexible;
        if flexible_integer {
            self.fit(node, value, EvalTy::I64)
        } else {
            Ok(value)
        }
    }

    fn float_operand(
        &mut self,
        operand: NodeId,
        partners: &[NodeId],
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let sig = some!(self.sig(operand, need));
        if sig.kind != Kind::Int || !is_float_ty(own) {
            return self.child(operand, own, ctx, need);
        }
        let flexible = sig.width == Width::Flexible;
        let mut partner = None;
        for other in partners.iter().filter(|partner| **partner != operand) {
            match self.known_sig(*other)?.width {
                Width::Fixed(scalar) if is_integer(scalar) => {
                    partner = Some(scalar);
                    break;
                }
                Width::Fixed(_) | Width::Flexible | Width::Widened(_) | Width::Adaptive => {}
            }
        }
        let ty = match partner {
            Some(scalar) if flexible => of_scalar(scalar),
            _ if flexible && !is_chosen_float(own) => {
                return self.child(operand, own, ctx, need);
            }
            _ => own_type(&sig, EvalTy::Exact),
        };
        let value = some!(self.child(operand, ty, ctx, need)?);
        if flexible && int_range(ty).is_some() {
            let value = self.fit(operand, value, ty)?;
            return Ok(Some(self.convert_integer(operand, false, value, own)?));
        }
        Ok(Some(self.integer_to_float(operand, value, own)?))
    }

    pub(in crate::semantics::analysis) fn integer_to_float(
        &mut self,
        node: NodeId,
        value: CVal,
        ty: EvalTy,
    ) -> Result<CVal, Internal> {
        let flexible = !matches!(self.known_sig(node)?.width, Width::Fixed(_));
        self.convert_integer(node, flexible, value, ty)
    }

    pub(super) fn convert_integer(
        &mut self,
        node: NodeId,
        flexible: bool,
        value: CVal,
        ty: EvalTy,
    ) -> Result<CVal, Internal> {
        let CVal::Int(integer) = value else {
            return Ok(value);
        };
        let converted = convert(value, ty);
        let exactly = match converted {
            CVal::F32(float) => represents(integer, f64::from(float)),
            CVal::F64(float) => represents(integer, float),
            CVal::Int(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_)
            | CVal::Array(_)
            | CVal::Error => true,
        };
        if flexible && !exactly {
            self.error_at_node(
                Code::DoesNotFit,
                node,
                format!(
                    "the integer {integer} converts to `{}` inexactly",
                    ty_name(ty)
                ),
            )?;
            return Ok(CVal::Error);
        }
        Ok(converted)
    }

    fn scaled(&mut self, node: NodeId, value: CVal, target: EvalTy) -> Result<CVal, Internal> {
        if matches!(self.node(node)?.op, Op::Number(..)) {
            return Ok(value);
        }
        let Some(scale) = self.memo.default_scales.get(&node).copied() else {
            return Ok(value);
        };
        map_leaves(value, |leaf| self.scaled_leaf(node, scale, leaf, target))
    }

    fn scaled_leaf(
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
}

fn int_position(value: CVal) -> Option<i128> {
    match value {
        CVal::Int(position) => Some(position),
        CVal::F32(_)
        | CVal::F64(_)
        | CVal::Bool(_)
        | CVal::Enum(..)
        | CVal::Str(_)
        | CVal::Node(_)
        | CVal::Array(_)
        | CVal::Error => None,
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

fn scaled_into(value: CVal, scale: f64, ty: EvalTy) -> Result<CVal, Internal> {
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
) -> Option<F> {
    Some(match op {
        BinaryOp::Add => a + b,
        BinaryOp::Sub => a - b,
        BinaryOp::Mul => a * b,
        BinaryOp::Div => a / b,
        BinaryOp::Rem => a.spec_rem(b),
        BinaryOp::Pow => a.spec_pow(b),
        BinaryOp::IntDiv
        | BinaryOp::Is
        | BinaryOp::IsNot
        | BinaryOp::Lt
        | BinaryOp::Le
        | BinaryOp::Gt
        | BinaryOp::Ge
        | BinaryOp::And
        | BinaryOp::Or => return None,
    })
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
        BinaryOp::Add
        | BinaryOp::Sub
        | BinaryOp::Mul
        | BinaryOp::Div
        | BinaryOp::IntDiv
        | BinaryOp::Rem
        | BinaryOp::Pow
        | BinaryOp::And
        | BinaryOp::Or => return CVal::Error,
    };
    CVal::Bool(result)
}
