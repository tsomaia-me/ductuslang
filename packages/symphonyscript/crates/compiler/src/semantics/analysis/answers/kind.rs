use crate::semantics::builtin::Builtin;
use symphonyscript_core::{BinaryOp, FamilyId, Scalar, UnaryOp, Value};

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::float_for::float_for;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::kind_of::kind_of;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::signature::Extra;
use crate::semantics::analysis::signature::arith_width;
use crate::semantics::analysis::signature::branch_width;
use crate::semantics::analysis::signature::branch_width_of;
use crate::semantics::analysis::signature::operand;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::unify_kinds::unify_kinds;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::{Number, Suffix};

fn float_width(width: Width) -> Width {
    match width {
        Width::Fixed(scalar) => Width::Fixed(float_for(scalar)),
        other => other,
    }
}

fn numeric_kind(a: Kind, b: Kind) -> Kind {
    match (a, b) {
        (Kind::Int, Kind::Int) => Kind::Int,
        (Kind::Any, other) | (other, Kind::Any) => other,
        _ => Kind::Float,
    }
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn sig_extra(
        &mut self,
        node: NodeId,
        op: &Op,
        need: &mut Vec<Query>,
    ) -> Result<Option<Extra>, Internal> {
        Ok(match op {
            Op::Read(binding) => {
                let init = self.binding(*binding)?.init;
                self.sig(init, need).map(Extra::Read)
            }
            Op::ToRead { wire, attrs, .. } => self.destination_read_reactive(*wire, attrs, need),
            Op::History {
                cell: crate::semantics::analysis::history_cell::HistoryCell::Input(input),
                ..
            } => match self.bound(Query::Input(*input), need) {
                Some(bound) => match self.first_binding(&bound) {
                    Some(binding) => {
                        let init = self.binding(binding)?.init;
                        let record = self.binding(binding)?.clone();
                        self.sig(init, need).map(|init| {
                            let kind = match record.annotation {
                                Some(annotation) => kind_of(annotation),
                                None => self.cell_kind(&record, init.kind),
                            };
                            Extra::History(Sig::of(kind, Width::Adaptive, true, init.family))
                        })
                    }
                    None => Some(Extra::History(Sig::ERROR)),
                },
                None => None,
            },
            Op::History { .. } => self.history_sig(node, need)?.map(|found| match found {
                Some(sig) => Extra::History(sig),
                None => Extra::None,
            }),
            Op::If(_, then, otherwise) => self.arm_forms(node, &[*then, *otherwise], need)?,
            Op::Match {
                arms, otherwise, ..
            } => {
                let values: Vec<NodeId> = arms
                    .iter()
                    .map(|(_, value)| *value)
                    .chain(*otherwise)
                    .collect();
                self.arm_forms(node, &values, need)?
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
            | Op::Use(_)
            | Op::Chosen(_)
            | Op::Arg(_)
            | Op::Slot(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Pick { .. }
            | Op::Element { .. }
            | Op::Array(_)
            | Op::Convert(..)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::Stepped(_)
            | Op::Native { .. } => Some(Extra::None),
        })
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

    pub(in crate::semantics::analysis) fn synth_of(
        &mut self,
        op: &Op,
        sig: &Sig,
    ) -> Result<Option<Scalar>, Internal> {
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
                Builtin::Capacity => None,
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
            | Op::Slot(_)
            | Op::Recursive { .. } => None,
            Op::Stepped(_) => Some(Scalar::Bool),
            Op::For(for_node) => self.combine(&[for_node.start, for_node.body])?,
        })
    }

    pub(in crate::semantics::analysis) fn is_string_bytes(
        &self,
        node: NodeId,
    ) -> Result<bool, Internal> {
        let mut leaves = vec![node];
        while let Some(leaf) = leaves.pop() {
            crate::semantics::counted_walk::CountedWalk::ArrayChecksIsStringBytes.step();
            match &self.node(leaf)?.op {
                Op::Use(inner) | Op::Chosen(inner) => leaves.push(*inner),
                Op::If(_, then, otherwise) => leaves.extend([*then, *otherwise]),
                Op::Match {
                    arms, otherwise, ..
                } => leaves.extend(arms.iter().map(|(_, value)| *value).chain(*otherwise)),
                Op::Builtin(Builtin::Bytes, operands) => {
                    let [operand] = operands.as_slice() else {
                        return Ok(false);
                    };
                    if !self
                        .memo
                        .sigs
                        .get(operand)
                        .is_some_and(|sig| sig.kind == Kind::Str)
                    {
                        return Ok(false);
                    }
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
                | Op::Read(_)
                | Op::Arg(_)
                | Op::Slot(_)
                | Op::Unary(..)
                | Op::Binary(..)
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
                | Op::Native { .. } => return Ok(false),
            }
        }
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn holds_a_float_cell(
        &self,
        node: NodeId,
    ) -> Result<bool, Internal> {
        let Op::Array(elements) = &self.node(node)?.op else {
            return Ok(false);
        };
        let mut pending = elements.clone();
        while let Some(element) = pending.pop() {
            crate::semantics::counted_walk::CountedWalk::CheckHoldsAFloatCell.step();
            match &self.node(self.through_uses(element)?)?.op {
                Op::Array(inner) => pending.extend(inner.iter().copied()),
                Op::Read(_) | Op::History { .. } => {
                    if self.known_sig(element)?.kind == Kind::Float {
                        return Ok(true);
                    }
                }
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Stepped(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Bare(_)
                | Op::Pi
                | Op::NodeRef(_)
                | Op::Use(_)
                | Op::Chosen(_)
                | Op::Arg(_)
                | Op::Slot(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::If(..)
                | Op::Match { .. }
                | Op::Pick { .. }
                | Op::Element { .. }
                | Op::Convert(..)
                | Op::Annotate(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::For(_)
                | Op::ToRead { .. }
                | Op::Join(..)
                | Op::Slice { .. }
                | Op::Native { .. } => {}
            }
        }
        Ok(false)
    }

    pub(in crate::semantics::analysis) fn holds_no_instance(
        &self,
        node: NodeId,
    ) -> Result<bool, Internal> {
        let mut node = node;
        loop {
            node = match &self.node(node)?.op {
                Op::Chosen(inner) | Op::Use(inner) => *inner,
                Op::Read(binding) => {
                    return Ok(matches!(
                        self.binding(*binding)?.kind,
                        BindingKind::Input { .. } | BindingKind::State { .. }
                    ));
                }
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Pi => return Ok(true),
                Op::Bare(_)
                | Op::NodeRef(_)
                | Op::Arg(_)
                | Op::Slot(_)
                | Op::Unary(..)
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
                | Op::Native { .. } => return Ok(false),
            };
        }
    }

    pub(in crate::semantics::analysis) fn kept_type(
        &mut self,
        node: NodeId,
        op: &Op,
        sigs: &[Sig],
        group: Option<Sig>,
    ) -> Result<Option<Sig>, Internal> {
        let reactive = sigs.iter().any(|operand| operand.reactive);
        Ok(match op {
            Op::Binary(
                BinaryOp::And
                | BinaryOp::Or
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge
                | BinaryOp::Is
                | BinaryOp::IsNot,
                ..,
            )
            | Op::Unary(UnaryOp::Not, _) => Some(Sig::of(
                Kind::Bool,
                Width::Fixed(Scalar::Bool),
                reactive,
                None,
            )),
            Op::Convert(target, _) => Some(Sig::of(
                kind_of(*target),
                Width::Fixed(*target),
                reactive,
                None,
            )),
            Op::Pick { items, .. } => {
                let item_sigs = sigs.get(1..).unwrap_or_default();
                if item_sigs.iter().any(Sig::is_error)
                    || group.is_some_and(|group| group.is_error())
                {
                    return Ok(None);
                }
                self.elements_sig(node, items, item_sigs)?
                    .map(|(kind, width, family)| Sig::of(kind, width, reactive, family))
            }
            Op::Element { .. } => match sigs.first() {
                Some(base) if !base.is_error() => {
                    Some(Sig::of(base.kind, base.width, reactive, base.family))
                }
                Some(_) | None => None,
            },
            Op::Binary(..)
            | Op::Unary(..)
            | Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Read(_)
            | Op::Use(_)
            | Op::Chosen(_)
            | Op::Arg(_)
            | Op::Slot(_)
            | Op::If(..)
            | Op::Match { .. }
            | Op::Array(_)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::History { .. }
            | Op::Stepped(_)
            | Op::Native { .. } => None,
        })
    }

    pub(in crate::semantics::analysis) fn rule(
        &mut self,
        node: NodeId,
        op: &Op,
        operands: &[NodeId],
        sigs: &[Sig],
        extra: Extra,
    ) -> Result<Sig, Internal> {
        let reactive = sigs.iter().any(|sig| sig.reactive);
        let at = |position| operand(sigs, position);
        Ok(match op {
            Op::Number(number, suffix) => self.number_sig(number, *suffix)?,
            Op::Const(value) => {
                let scalar = match value {
                    Value::U8(_) => Scalar::U8,
                    Value::I32(_) => Scalar::I32,
                    Value::I64(_) => Scalar::I64,
                    Value::F32(_) => Scalar::F32,
                    Value::F64(_) => Scalar::F64,
                    Value::Bool(_) => Scalar::Bool,
                    Value::Enum(id, _) => Scalar::Enum(*id),
                    Value::Str(_) => Scalar::Str,
                };
                Sig::of(kind_of(scalar), Width::Fixed(scalar), false, None)
            }
            Op::Bool(_) => Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), false, None),
            Op::Stepped(_) => Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), true, None),
            Op::Str(_) => Sig::of(Kind::Str, Width::Fixed(Scalar::Str), false, None),
            Op::Concat(_) => self.concat_sig(operands, sigs)?,
            Op::Variant(id, _) => Sig::of(
                Kind::Enum(*id),
                Width::Fixed(Scalar::Enum(*id)),
                false,
                None,
            ),
            Op::Bare(_) => Sig::of(Kind::Bare, Width::Flexible, false, None),
            Op::Pi => Sig::of(Kind::Float, Width::Flexible, false, None),
            Op::NodeRef(_) => Sig::of(Kind::Node, Width::Flexible, false, None),
            Op::Use(_) | Op::Chosen(_) => at(0)?,
            Op::Arg(_) => return Err(Internal::missed("argument signature")),
            Op::Slot(slot) => self.slot(*slot)?.sig,
            Op::For(for_node) => self.for_sig(node, for_node, sigs)?,
            Op::Read(binding) => match extra {
                Extra::Read(init) => self.read_sig(*binding, init)?,
                Extra::None | Extra::ToRead { .. } | Extra::History(_) | Extra::Arms(_) => {
                    return Err(Internal::missed("read signature"));
                }
            },
            Op::History { fallback, .. } => match (fallback, extra) {
                (_, Extra::History(sig)) => sig,
                (Some(_), _) => {
                    let fallback = at(0)?;
                    Sig::of(fallback.kind, Width::Adaptive, true, fallback.family)
                }
                (None, Extra::None) => Sig::of(Kind::Any, Width::Adaptive, true, None),
                (None, Extra::Read(_) | Extra::ToRead { .. } | Extra::Arms(_)) => {
                    return Err(Internal::missed("history signature"));
                }
            },
            Op::Join(lhs, rhs) => {
                let Some((kind, width, family)) = self.elements_sig(node, &[*lhs, *rhs], sigs)?
                else {
                    return Ok(Sig::ERROR);
                };
                Sig::of(kind, width, reactive, family)
            }
            Op::Slice { lower, upper, .. } => {
                for (bound, _) in lower.iter().chain(upper.iter()) {
                    let bound_sig = self.known_sig(*bound)?;
                    if !matches!(bound_sig.kind, Kind::Int | Kind::Any) {
                        self.error_at_node(Code::IndexType, *bound, "a slice bound is an integer")?;
                        return Ok(Sig::ERROR);
                    }
                    if bound_sig.family.is_some() {
                        self.error_at_node(
                            Code::UnitOperation,
                            *bound,
                            "a slice bound is a plain number",
                        )?;
                        return Ok(Sig::ERROR);
                    }
                }
                let base = at(0)?;
                Sig::of(base.kind, base.width, reactive, base.family)
            }
            Op::Native { native, args } => self.native_sig(node, *native, args, sigs)?,
            Op::Unary(UnaryOp::Neg, _) => {
                let first = at(0)?;
                if !first.is_numeric() {
                    self.error_at_node(Code::OperandType, node, "unary `-` takes a number")?;
                    return Ok(Sig::ERROR);
                }
                first
            }
            Op::Unary(UnaryOp::Not, _) => {
                if !matches!(at(0)?.kind, Kind::Bool | Kind::Any) {
                    self.error_at_node(Code::OperandType, node, "`not` takes a `bool`")?;
                    return Ok(self.kept_bool(node, reactive));
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
            Op::Binary(op, lhs, rhs) => {
                self.binary_sig(node, *op, (*lhs, at(0)?), (*rhs, at(1)?))?
            }
            Op::If(condition, then, otherwise) => {
                let (second, third) = (at(1)?, at(2)?);
                if !matches!(at(0)?.kind, Kind::Bool | Kind::Any) {
                    self.error_at_node(
                        Code::ConditionType,
                        *condition,
                        "the condition is a `bool`",
                    )?;
                    return Ok(Sig::ERROR);
                }
                let Some(kind) = unify_kinds(second.kind, third.kind) else {
                    self.error_at_node(
                        Code::TypeMismatch,
                        node,
                        "the branches of an `if` hold values of different types",
                    )?;
                    return Ok(Sig::ERROR);
                };
                let Some(family) =
                    self.one_family(node, &[(*then, second), (*otherwise, third)])?
                else {
                    return Ok(Sig::ERROR);
                };
                if !arms_alike(&extra) {
                    self.error_at_node(Code::TypeMismatch, node, ARM_SHAPES)?;
                    return Ok(Sig::ERROR);
                }
                let width = branch_width(kind, &second, &third);
                Sig::of(kind, width, reactive, family)
            }
            Op::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let sig = self.match_sig(node, (*scrutinee, arms, *otherwise), sigs)?;
                if !sig.is_error() && !arms_alike(&extra) {
                    self.error_at_node(Code::TypeMismatch, node, ARM_SHAPES)?;
                    return Ok(Sig::ERROR);
                }
                sig
            }
            Op::Pick { index, items } => {
                let first = at(0)?;
                if !matches!(first.kind, Kind::Int | Kind::Any) {
                    self.error_at_node(Code::IndexType, *index, "an index is an integer")?;
                }
                let item_sigs = sigs.get(1..).ok_or(Internal::missed("operand signature"))?;
                let Some((kind, width, family)) = self.elements_sig(node, items, item_sigs)? else {
                    return Ok(Sig::ERROR);
                };
                if !matches!(first.kind, Kind::Int | Kind::Any) {
                    self.memo.kept_types.insert(node);
                    return Ok(Sig::of(kind, width, reactive, family));
                }
                let width = if first.reactive {
                    Width::Adaptive
                } else {
                    width
                };
                Sig::of(kind, width, reactive, family)
            }
            Op::Element { index, .. } => {
                let (first, second) = (at(0)?, at(1)?);
                if !matches!(second.kind, Kind::Int | Kind::Any) {
                    self.error_at_node(Code::IndexType, *index, "an index is an integer")?;
                    if first.is_error() {
                        return Ok(Sig::ERROR);
                    }
                    self.memo.kept_types.insert(node);
                    return Ok(Sig::of(first.kind, first.width, reactive, first.family));
                }
                let width = if second.reactive {
                    Width::Adaptive
                } else {
                    first.width
                };
                Sig::of(first.kind, width, reactive, first.family)
            }
            Op::Array(elements) => {
                let Some((kind, width, family)) = self.elements_sig(node, elements, sigs)? else {
                    return Ok(Sig::ERROR);
                };
                Sig::of(kind, width, reactive, family)
            }
            Op::Convert(target, _) => {
                let first = at(0)?;
                if !first.is_numeric() {
                    let message = match first.kind {
                        Kind::Bool | Kind::Enum(_) | Kind::Bare => {
                            "a `bool` or an enum value is not converted"
                        }
                        Kind::Int
                        | Kind::Float
                        | Kind::Str
                        | Kind::Node
                        | Kind::Any
                        | Kind::Error => "a conversion takes a number",
                    };
                    self.error_at_node(Code::ConversionType, node, message)?;
                    if matches!(first.kind, Kind::Any | Kind::Error) {
                        return Ok(Sig::ERROR);
                    }
                    self.memo.kept_types.insert(node);
                }
                Sig::of(
                    kind_of(*target),
                    Width::Fixed(*target),
                    reactive,
                    first.family,
                )
            }
            Op::Annotate(target, _) => Sig::of(
                kind_of(*target),
                Width::Fixed(*target),
                reactive,
                at(0)?.family,
            ),
            Op::Builtin(builtin, _) => self.builtin_sig(node, *builtin, operands, sigs)?,
            Op::Recursive { function, .. } if self.hir.fn_decl(*function)?.faulty_head => {
                Sig::ERROR
            }
            Op::Recursive { function, .. } => {
                let reading = format!(
                    "this call of `{}` reads a cell",
                    self.function_name(*function)?
                );
                self.reactive_recursion(node, *function, reading)?;
                Sig::ERROR
            }
            Op::ToRead { attrs, .. } => self.destination_read_sig(node, attrs, extra)?,
        })
    }

    pub(in crate::semantics::analysis) fn elements_sig(
        &mut self,
        node: NodeId,
        items: &[NodeId],
        item_sigs: &[Sig],
    ) -> Result<Option<(Kind, Width, Option<FamilyId>)>, Internal> {
        if items.len() != item_sigs.len() {
            return Err(Internal::missed("operand signature"));
        }
        let Some(first) = item_sigs.first().copied() else {
            return Ok(Some((Kind::Any, Width::Flexible, None)));
        };
        let mut kind = first.kind;
        let mut width = first.width;
        for sig in item_sigs.iter().skip(1) {
            let Some(unified) = unify_kinds(kind, sig.kind) else {
                self.error_at_node(
                    Code::TypeMismatch,
                    node,
                    "the elements of the array hold values of different types",
                )?;
                return Ok(None);
            };
            kind = unified;
            width = branch_width_of(kind, width, sig.width);
        }
        let pairs: Vec<(NodeId, Sig)> = items
            .iter()
            .copied()
            .zip(item_sigs.iter().copied())
            .collect();
        let family = some!(self.one_family(node, &pairs)?);
        Ok(Some((kind, width, family)))
    }

    fn number_sig(&self, number: &Number, suffix: Suffix) -> Result<Sig, Internal> {
        let literal_kind = match number {
            Number::Int(_) => Kind::Int,
            Number::Float(_) => Kind::Float,
        };
        Ok(match suffix {
            Suffix::None => Sig::of(literal_kind, Width::Flexible, false, None),
            Suffix::Type(scalar) => Sig::of(kind_of(scalar), Width::Fixed(scalar), false, None),
            Suffix::Unit { family, unit } => {
                let integral = self.unit_decl(family, unit)?.scale.fract() == 0.0;
                let kind = if literal_kind == Kind::Int && integral {
                    Kind::Int
                } else {
                    Kind::Float
                };
                Sig::of(kind, Width::Flexible, false, Some(family))
            }
        })
    }

    fn concat_sig(&mut self, operands: &[NodeId], sigs: &[Sig]) -> Result<Sig, Internal> {
        for (operand, sig) in operands.iter().zip(sigs) {
            if sig.reactive {
                let reads = self.untypable_reads(*operand)?;
                self.typing.read_by_faults.extend(reads);
                let record = self.node(*operand)?;
                let (loc, scope) = (record.loc, record.scope);
                self.error(
                    Code::ReactiveString,
                    loc,
                    scope,
                    "a string exists only at compile time, and this interpolation reads a cell",
                )?;
                return Ok(Sig::ERROR);
            }
            let named = matches!(self.node(*operand)?.op, Op::Use(_));
            if sig.kind == Kind::Node || (sig.kind == Kind::Bare && !named) {
                let message = if sig.kind == Kind::Node {
                    "an interpolation holds a number, a `bool`, an enum value or a string, not an instance"
                } else {
                    "a variant in an interpolation is written `Enum::Variant`"
                };
                self.error_at_node(Code::InterpolationType, *operand, message)?;
                return Ok(Sig::ERROR);
            }
        }
        Ok(Sig::of(Kind::Str, Width::Fixed(Scalar::Str), false, None))
    }

    fn binary_sig(
        &mut self,
        node: NodeId,
        op: BinaryOp,
        lhs: (NodeId, Sig),
        rhs: (NodeId, Sig),
    ) -> Result<Sig, Internal> {
        let (a, b) = (lhs.1, rhs.1);
        let reactive = a.reactive || b.reactive;
        let named = matches!(self.node(lhs.0)?.op, Op::Use(_))
            || matches!(self.node(rhs.0)?.op, Op::Use(_));
        let numeric = a.is_numeric() && b.is_numeric();
        Ok(match op {
            BinaryOp::Add if concatenates(&a, &b) => {
                if reactive {
                    self.error_at_node(
                        Code::ReactiveString,
                        node,
                        "a string exists only at compile time, and this one reads a cell",
                    )?;
                    return Ok(Sig::ERROR);
                }
                Sig::of(Kind::Str, Width::Fixed(Scalar::Str), false, None)
            }
            BinaryOp::Add if !numeric && self.memo.deferred_sums.contains(&node) => {
                let Some((kind, width, family)) =
                    self.elements_sig(node, &[lhs.0, rhs.0], &[a, b])?
                else {
                    return Ok(Sig::ERROR);
                };
                Sig::of(kind, width, reactive, family)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Rem | BinaryOp::Pow => {
                if !numeric {
                    self.error_at_node(Code::OperandType, node, Self::operand_type_message(op))?;
                    return Ok(Sig::ERROR);
                }
                let family = match op {
                    BinaryOp::Mul => match (a.family, b.family) {
                        (Some(_), Some(_)) => {
                            self.error_at_node(
                                Code::UnitOperation,
                                node,
                                "two unit values are not multiplied",
                            )?;
                            return Ok(Sig::ERROR);
                        }
                        (family, None) | (None, family) => family,
                    },
                    BinaryOp::Pow => {
                        if a.family.is_some() || b.family.is_some() {
                            self.error_at_node(
                                Code::UnitOperation,
                                node,
                                "`**` takes no unit value",
                            )?;
                            return Ok(Sig::ERROR);
                        }
                        None
                    }
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Div
                    | BinaryOp::IntDiv
                    | BinaryOp::Rem
                    | BinaryOp::Is
                    | BinaryOp::IsNot
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge
                    | BinaryOp::And
                    | BinaryOp::Or => match self.one_family(node, &[lhs, rhs])? {
                        Some(family) => family,
                        None => return Ok(Sig::ERROR),
                    },
                };
                Sig::of(
                    numeric_kind(a.kind, b.kind),
                    arith_width(&a, &b),
                    reactive,
                    family,
                )
            }
            BinaryOp::Div | BinaryOp::IntDiv => {
                let integers = matches!(a.kind, Kind::Int | Kind::Any)
                    && matches!(b.kind, Kind::Int | Kind::Any);
                if !numeric || (op == BinaryOp::IntDiv && !integers) {
                    self.error_at_node(Code::OperandType, node, Self::operand_type_message(op))?;
                    return Ok(Sig::ERROR);
                }
                let family = match (a.family, b.family) {
                    (Some(x), Some(y)) if x == y => None,
                    (Some(_), Some(_)) => {
                        self.error_at_node(
                            Code::MixedFamilies,
                            node,
                            "these values belong to different unit families",
                        )?;
                        return Ok(Sig::ERROR);
                    }
                    (Some(family), None) => Some(family),
                    (None, Some(_)) => {
                        self.error_at_node(
                            Code::UnitOperation,
                            node,
                            "a plain number is not divided by a unit value",
                        )?;
                        return Ok(Sig::ERROR);
                    }
                    (None, None) => None,
                };
                if op == BinaryOp::IntDiv {
                    return Ok(Sig::of(Kind::Int, arith_width(&a, &b), reactive, family));
                }
                let width = match arith_width(&a, &b) {
                    Width::Fixed(scalar) => Width::Fixed(float_for(scalar)),
                    other => other,
                };
                Sig::of(Kind::Float, width, reactive, family)
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                if !numeric {
                    self.error_at_node(Code::OperandType, node, Self::operand_type_message(op))?;
                    return Ok(self.kept_bool(node, reactive));
                }
                if self.one_family(node, &[lhs, rhs])?.is_none() {
                    return Ok(self.kept_bool(node, reactive));
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
            BinaryOp::Is | BinaryOp::IsNot => {
                let comparable = numeric
                    || match (a.kind, b.kind) {
                        (Kind::Any, _) | (_, Kind::Any) => true,
                        (Kind::Bool, Kind::Bool) | (Kind::Str, Kind::Str) => true,
                        (Kind::Enum(x), Kind::Enum(y)) => x == y,
                        (Kind::Enum(_), Kind::Bare) | (Kind::Bare, Kind::Enum(_)) => true,
                        (Kind::Bare, Kind::Bare) => reactive || named,
                        _ => false,
                    };
                if !comparable {
                    self.error_at_node(Code::OperandType, node, Self::operand_type_message(op))?;
                    return Ok(self.kept_bool(node, reactive));
                }
                if (a.kind == Kind::Str || b.kind == Kind::Str) && reactive {
                    self.error_at_node(
                        Code::ReactiveString,
                        node,
                        "a string exists only at compile time, and this comparison of strings reads a cell",
                    )?;
                    return Ok(self.kept_bool(node, reactive));
                }
                if numeric && self.one_family(node, &[lhs, rhs])?.is_none() {
                    return Ok(self.kept_bool(node, reactive));
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
            BinaryOp::And | BinaryOp::Or => {
                let logical = |sig: &Sig| matches!(sig.kind, Kind::Bool | Kind::Any);
                match (logical(&a), logical(&b)) {
                    (true, true) => {}
                    (false, false) => {
                        self.error_at_node(
                            Code::OperandType,
                            node,
                            Self::operand_type_message(op),
                        )?;
                        return Ok(self.kept_bool(node, reactive));
                    }
                    (true, false) | (false, true) => {
                        let refused = if logical(&a) { rhs.0 } else { lhs.0 };
                        self.error_at_node(
                            Code::TypeMismatch,
                            refused,
                            "this value has the wrong type here; `bool` is expected",
                        )?;
                        return Ok(self.kept_bool(node, reactive));
                    }
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
        })
    }

    fn kept_bool(&mut self, node: NodeId, reactive: bool) -> Sig {
        self.memo.kept_types.insert(node);
        Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
    }

    pub(in crate::semantics::analysis) fn operand_type_message(op: BinaryOp) -> &'static str {
        match op {
            BinaryOp::Add => "`+` takes two numbers or two strings",
            BinaryOp::Div => "`/` takes numbers",
            BinaryOp::IntDiv => "`div` takes integers",
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                "`<`, `<=`, `>` and `>=` compare numbers"
            }
            BinaryOp::Is | BinaryOp::IsNot => {
                "`is` compares two numbers, `bool`s, values of one enum or strings"
            }
            BinaryOp::And | BinaryOp::Or => "`and` and `or` take `bool`s",
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Rem | BinaryOp::Pow => {
                "this operator takes numbers"
            }
        }
    }

    fn builtin_sig(
        &mut self,
        node: NodeId,
        builtin: Builtin,
        operands: &[NodeId],
        sigs: &[Sig],
    ) -> Result<Sig, Internal> {
        let reactive = sigs.iter().any(|sig| sig.reactive);
        match builtin {
            Builtin::Capacity => {
                return Ok(Sig::of(Kind::Int, Width::Flexible, false, None));
            }
            Builtin::Len => {
                return Ok(Sig::of(
                    Kind::Int,
                    Width::Fixed(Scalar::I32),
                    reactive,
                    None,
                ));
            }
            Builtin::Bytes => {
                let first = operand(sigs, 0)?;
                let array = operands
                    .first()
                    .is_some_and(|operand| self.arrays.contains(operand));
                if !matches!(first.kind, Kind::Int | Kind::Str | Kind::Any)
                    || (first.kind != Kind::Str && !array)
                {
                    self.error_at_node(
                        Code::OperandType,
                        node,
                        "`bytes` takes a string or an integer array",
                    )?;
                    return Ok(Sig::ERROR);
                }
                return Ok(Sig::of(Kind::Int, Width::Fixed(Scalar::U8), reactive, None));
            }
            Builtin::Abs
            | Builtin::Min
            | Builtin::Max
            | Builtin::Clamp
            | Builtin::Floor
            | Builtin::Ceil
            | Builtin::Round
            | Builtin::Sqrt
            | Builtin::Sin
            | Builtin::Cos
            | Builtin::Tan
            | Builtin::Exp
            | Builtin::Log
            | Builtin::Lerp
            | Builtin::Rand => {}
        }
        if sigs.iter().any(|sig| !sig.is_numeric()) {
            self.error_at_node(
                Code::OperandType,
                node,
                format!("`{}` takes numbers", builtin.name()),
            )?;
            return Ok(Sig::ERROR);
        }
        let pairs: Vec<(NodeId, Sig)> =
            operands.iter().copied().zip(sigs.iter().copied()).collect();
        let first = operand(sigs, 0)?;
        let combined = sigs.iter().skip(1).fold(first, |acc, sig| {
            Sig::of(
                numeric_kind(acc.kind, sig.kind),
                arith_width(&acc, sig),
                reactive,
                None,
            )
        });
        Ok(match builtin {
            Builtin::Abs => first,
            Builtin::Min | Builtin::Max | Builtin::Clamp => {
                let Some(family) = self.one_family(node, &pairs)? else {
                    return Ok(Sig::ERROR);
                };
                Sig::of(combined.kind, combined.width, reactive, family)
            }
            Builtin::Floor | Builtin::Ceil | Builtin::Round => Sig::of(
                Kind::Float,
                float_width(first.width),
                reactive,
                first.family,
            ),
            Builtin::Lerp => {
                let interval = pairs
                    .get(..2)
                    .ok_or(Internal::missed("operand signature"))?;
                let Some(family) = self.one_family(node, interval)? else {
                    return Ok(Sig::ERROR);
                };
                if operand(sigs, 2)?.family.is_some() {
                    self.error_at_node(
                        Code::UnitOperation,
                        node,
                        "the third argument of `lerp` is a plain number",
                    )?;
                    return Ok(Sig::ERROR);
                }
                Sig::of(Kind::Float, float_width(combined.width), reactive, family)
            }
            Builtin::Rand => {
                if sigs
                    .iter()
                    .any(|sig| !matches!(sig.kind, Kind::Int | Kind::Any))
                {
                    self.error_at_node(Code::OperandType, node, "`rand` takes two integers")?;
                    return Ok(Sig::ERROR);
                }
                if sigs.iter().any(|sig| sig.family.is_some()) {
                    self.error_at_node(Code::UnitOperation, node, "`rand` takes plain numbers")?;
                    return Ok(Sig::ERROR);
                }
                Sig::of(Kind::Float, Width::Fixed(Scalar::F32), reactive, None)
            }
            Builtin::Len | Builtin::Capacity | Builtin::Bytes => {
                return Err(Internal::missed("array built-in"));
            }
            Builtin::Sqrt
            | Builtin::Sin
            | Builtin::Cos
            | Builtin::Tan
            | Builtin::Exp
            | Builtin::Log => {
                if sigs.iter().any(|sig| sig.family.is_some()) {
                    self.error_at_node(
                        Code::UnitOperation,
                        node,
                        format!("`{}` takes a plain number", builtin.name()),
                    )?;
                    return Ok(Sig::ERROR);
                }
                Sig::of(Kind::Float, float_width(combined.width), reactive, None)
            }
        })
    }

    fn native_sig(
        &mut self,
        node: NodeId,
        native: symphonyscript_core::NativeId,
        args: &[NodeId],
        sigs: &[Sig],
    ) -> Result<Sig, Internal> {
        let reactive = sigs.iter().any(|sig| sig.reactive);
        let decl = self
            .vocabulary
            .native(native)
            .ok_or(Internal::missed("native function"))?;
        let signature = &decl.signature;
        let types = signature
            .params
            .iter()
            .chain(std::iter::once(&signature.result));
        let takes_text = types
            .map(crate::semantics::analysis::evaluation::native_ty)
            .any(|ty| ty.scalar == Scalar::Str);
        if takes_text && reactive {
            self.error_at_node(
                Code::CompileTimeNative,
                node,
                format!(
                    "`{}` takes or returns a string, so it runs only at compile time, and an argument here reads a cell",
                    decl.name
                ),
            )?;
            return Ok(Sig::ERROR);
        }
        if let Some((arg, _)) = args.iter().zip(sigs).find(|(_, sig)| sig.family.is_some()) {
            self.error_at_node(
                Code::UnitOperation,
                *arg,
                format!("`{}` takes plain values", decl.name),
            )?;
            return Ok(Sig::ERROR);
        }
        let scalar = crate::semantics::analysis::evaluation::native_ty(&signature.result).scalar;
        Ok(Sig::of(
            kind_of(scalar),
            Width::Fixed(scalar),
            reactive,
            None,
        ))
    }

    fn destination_read_sig(
        &mut self,
        node: NodeId,
        attrs: &[(
            crate::semantics::analysis::item_ref::ItemRef,
            symphonyscript_core::AttrId,
        )],
        extra: Extra,
    ) -> Result<Sig, Internal> {
        let reactive = matches!(extra, Extra::ToRead { reactive: true });
        let mut found = Vec::new();
        for (item, attr) in attrs {
            let decl = self
                .attr_decl(*item, *attr)?
                .ok_or(Internal::missed("attribute"))?;
            found.push((*item, decl.clone()));
        }
        let mut decls = found.into_iter();
        let (first_item, first) = decls.next().ok_or(Internal::missed("attribute"))?;
        let family: Option<FamilyId> = first.unit.map(|unit| unit.family);
        for (item, decl) in decls {
            if decl.ty != first.ty || decl.unit.map(|unit| unit.family) != family {
                let (one, other) = (
                    self.item_description(first_item)?,
                    self.item_description(item)?,
                );
                self.error_at_node(
                    Code::ToCandidates,
                    node,
                    format!(
                        "the destination candidates hold `{}` with different types or unit families, {one} and {other} among them, and a `to` read takes one",
                        first.name
                    ),
                )?;
                return Ok(Sig::ERROR);
            }
        }
        let scalar = first.ty.scalar;
        let width = if reactive {
            Width::Adaptive
        } else {
            Width::Fixed(scalar)
        };
        Ok(Sig::of(kind_of(scalar), width, reactive, family))
    }
}

fn concatenates(a: &Sig, b: &Sig) -> bool {
    let string_or_open = |sig: &Sig| sig.kind == Kind::Str || sig.open;
    string_or_open(a) && string_or_open(b) && (a.kind == Kind::Str || b.kind == Kind::Str)
}

fn arms_alike(extra: &Extra) -> bool {
    let Extra::Arms(forms) = extra else {
        return true;
    };
    let arrays: Vec<bool> = forms.iter().filter_map(|(array, _)| *array).collect();
    if arrays.iter().any(|array| *array) && arrays.iter().any(|array| !*array) {
        return false;
    }
    let mut rows = forms.iter().filter_map(|(_, rows)| rows.as_ref());
    let Some(first) = rows.next() else {
        return true;
    };
    rows.all(|other| other.len() == first.len())
}

const ARM_SHAPES: &str = "the branches hold arrays of different shapes";

pub(crate) fn joins(op: &Op) -> bool {
    match op {
        Op::Use(_)
        | Op::Chosen(_)
        | Op::Unary(UnaryOp::Neg, _)
        | Op::If(..)
        | Op::Match { .. }
        | Op::Pick { .. }
        | Op::Element { .. }
        | Op::Array(_)
        | Op::Convert(..)
        | Op::Annotate(..) => true,
        Op::Binary(op, _, _) => matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Rem
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge
                | BinaryOp::Is
                | BinaryOp::IsNot
        ),
        Op::Builtin(builtin, _) => matches!(
            builtin,
            Builtin::Abs
                | Builtin::Min
                | Builtin::Max
                | Builtin::Clamp
                | Builtin::Lerp
                | Builtin::Floor
                | Builtin::Ceil
                | Builtin::Round
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
        | Op::Recursive { .. }
        | Op::For(_)
        | Op::ToRead { .. }
        | Op::Join(..)
        | Op::Slice { .. }
        | Op::History { .. }
        | Op::Stepped(_)
        | Op::Native { .. } => false,
    }
}
