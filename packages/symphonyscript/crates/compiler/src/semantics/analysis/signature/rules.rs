use symphonyscript_core::{BinaryOp, Builtin, FamilyId, Scalar, UnaryOp, Value};

use super::arith_width::arith_width;
use super::branch_width::branch_width;
use super::branch_width_of::branch_width_of;
use super::extra::Extra;
use super::operand::operand;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::float_for::float_for;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::kind_of::kind_of;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::sig::Sig;
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
    pub(super) fn rule(
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
            Op::Use(_) => at(0)?,
            Op::Arg(_) => return Err(Internal::missed("argument signature")),
            Op::Read(binding) => match extra {
                Extra::Read(init) => self.read_sig(*binding, init)?,
                Extra::None | Extra::ToRead { .. } => {
                    return Err(Internal::missed("read signature"));
                }
            },
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
                    return Ok(Sig::ERROR);
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
                let width = branch_width(kind, &second, &third);
                Sig::of(kind, width, reactive, family)
            }
            Op::Match {
                scrutinee,
                arms,
                otherwise,
            } => self.match_sig(node, (*scrutinee, arms, *otherwise), sigs)?,
            Op::Pick { index, items } => {
                let first = at(0)?;
                if !matches!(first.kind, Kind::Int | Kind::Any) {
                    self.error_at_node(Code::IndexType, *index, "an index is an integer")?;
                    return Ok(Sig::ERROR);
                }
                let item_sigs = sigs.get(1..).ok_or(Internal::missed("operand signature"))?;
                let Some((kind, width, family)) = self.elements_sig(node, items, item_sigs)? else {
                    return Ok(Sig::ERROR);
                };
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
                    return Ok(Sig::ERROR);
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
                    return Ok(Sig::ERROR);
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
            Op::Recursive { .. } => {
                self.error_at_node(
                    Code::ReactiveRecursion,
                    node,
                    "a call to a recursive function has compile-time arguments, and this one reads a cell",
                )?;
                Sig::ERROR
            }
            Op::ToRead { attrs, .. } => self.destination_read_sig(node, attrs, extra)?,
        })
    }

    fn elements_sig(
        &mut self,
        node: NodeId,
        items: &[NodeId],
        item_sigs: &[Sig],
    ) -> Result<Option<(Kind, Width, Option<FamilyId>)>, Internal> {
        if items.len() != item_sigs.len() {
            return Err(Internal::missed("operand signature"));
        }
        let first = operand(item_sigs, 0)?;
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
                        "a string expression reads no cell",
                    )?;
                    return Ok(Sig::ERROR);
                }
                Sig::of(Kind::Str, Width::Fixed(Scalar::Str), false, None)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Rem | BinaryOp::Pow => {
                if !numeric {
                    let message = match op {
                        BinaryOp::Add => "`+` takes two numbers or two strings",
                        BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
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
                        | BinaryOp::Or => "this operator takes numbers",
                    };
                    self.error_at_node(Code::OperandType, node, message)?;
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
                    let message = if op == BinaryOp::IntDiv {
                        "`div` takes integers"
                    } else {
                        "`/` takes numbers"
                    };
                    self.error_at_node(Code::OperandType, node, message)?;
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
                    self.error_at_node(
                        Code::OperandType,
                        node,
                        "`<`, `<=`, `>` and `>=` compare numbers",
                    )?;
                    return Ok(Sig::ERROR);
                }
                if self.one_family(node, &[lhs, rhs])?.is_none() {
                    return Ok(Sig::ERROR);
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
                    self.error_at_node(
                        Code::OperandType,
                        node,
                        "`is` compares two numbers, `bool`s, values of one enum or strings",
                    )?;
                    return Ok(Sig::ERROR);
                }
                if (a.kind == Kind::Str || b.kind == Kind::Str) && reactive {
                    self.error_at_node(
                        Code::ReactiveString,
                        node,
                        "a string expression reads no cell",
                    )?;
                    return Ok(Sig::ERROR);
                }
                if numeric && self.one_family(node, &[lhs, rhs])?.is_none() {
                    return Ok(Sig::ERROR);
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
            BinaryOp::And | BinaryOp::Or => {
                if !matches!(a.kind, Kind::Bool | Kind::Any)
                    || !matches!(b.kind, Kind::Bool | Kind::Any)
                {
                    self.error_at_node(Code::OperandType, node, "`and` and `or` take `bool`s")?;
                    return Ok(Sig::ERROR);
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
        })
    }

    fn builtin_sig(
        &mut self,
        node: NodeId,
        builtin: Builtin,
        operands: &[NodeId],
        sigs: &[Sig],
    ) -> Result<Sig, Internal> {
        let reactive = sigs.iter().any(|sig| sig.reactive);
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
            found.push(decl.clone());
        }
        let mut decls = found.into_iter();
        let first = decls.next().ok_or(Internal::missed("attribute"))?;
        let family: Option<FamilyId> = first.unit.map(|unit| unit.family);
        for decl in decls {
            if decl.ty != first.ty || decl.unit.map(|unit| unit.family) != family {
                self.error_at_node(
                    Code::ToCandidates,
                    node,
                    "every destination candidate has this attribute with one type and unit family",
                )?;
                return Ok(Sig::ERROR);
            }
        }
        if !first.ty.is_scalar() {
            self.error_at_node(
                Code::ToCandidates,
                node,
                "`to` reads a single-valued attribute",
            )?;
            return Ok(Sig::ERROR);
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
