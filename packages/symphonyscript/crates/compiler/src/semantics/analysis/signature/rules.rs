use symphonyscript_core::{BinaryOp, Builtin, FamilyId, Scalar, UnaryOp, Value};

use super::run::Extra;
use crate::diagnostics::Code;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::sig::{
    Kind, Sig, Width, common, float_for, kind_of, meet_flexible, unify_kinds,
};
use crate::semantics::ir::{Number, Suffix};

pub(super) fn arith_width(a: &Sig, b: &Sig) -> Width {
    let settled = |width| match width {
        Width::Widened(scalar) => Width::Fixed(float_for(scalar)),
        other => other,
    };
    match (settled(a.width), settled(b.width)) {
        (Width::Adaptive, _) | (_, Width::Adaptive) => Width::Adaptive,
        (Width::Fixed(x), Width::Fixed(y)) => Width::Fixed(common(x, y).unwrap_or(x)),
        (Width::Fixed(x), Width::Flexible) => Width::Fixed(meet_flexible(x, b.kind)),
        (Width::Flexible, Width::Fixed(y)) => Width::Fixed(meet_flexible(y, a.kind)),
        (Width::Flexible, Width::Flexible) => Width::Flexible,
        (Width::Widened(x), _) | (_, Width::Widened(x)) => Width::Fixed(float_for(x)),
    }
}

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
    ) -> Sig {
        let reactive = sigs.iter().any(|sig| sig.reactive);
        let first = sigs.first().copied().unwrap_or(Sig::ERROR);
        let second = sigs.get(1).copied().unwrap_or(Sig::ERROR);
        match op {
            Op::Number(number, suffix) => self.number_sig(number, *suffix),
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
            Op::Concat(_) => self.concat_sig(operands, sigs),
            Op::Variant(id, _) => Sig::of(
                Kind::Enum(*id),
                Width::Fixed(Scalar::Enum(*id)),
                false,
                None,
            ),
            Op::Bare(_) => Sig::of(Kind::Bare, Width::Flexible, false, None),
            Op::Pi => Sig::of(Kind::Float, Width::Flexible, false, None),
            Op::NodeRef(_) => Sig::of(Kind::Node, Width::Flexible, false, None),
            Op::Use(_) => first,
            Op::Arg(_) => Sig::ERROR,
            Op::Read(binding) => match extra {
                Extra::Read(init) => self.read_sig(*binding, init),
                Extra::None | Extra::ToRead { .. } => Sig::ERROR,
            },
            Op::Unary(UnaryOp::Neg, _) => {
                if !first.is_numeric() {
                    self.error_at_node(Code::OperandType, node, "unary `-` takes a number");
                    return Sig::ERROR;
                }
                first
            }
            Op::Unary(UnaryOp::Not, _) => {
                if !matches!(first.kind, Kind::Bool | Kind::Any) {
                    self.error_at_node(Code::OperandType, node, "`not` takes a `bool`");
                    return Sig::ERROR;
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
            Op::Binary(op, lhs, rhs) => self.binary_sig(node, *op, (*lhs, first), (*rhs, second)),
            Op::If(_, then, otherwise) => {
                if !matches!(first.kind, Kind::Bool | Kind::Any) {
                    let condition = operands.first().copied().unwrap_or(node);
                    self.error_at_node(Code::ConditionType, condition, "the condition is a `bool`");
                    return Sig::ERROR;
                }
                let third = sigs.get(2).copied().unwrap_or(Sig::ERROR);
                let Some(kind) = unify_kinds(second.kind, third.kind) else {
                    self.error_at_node(
                        Code::TypeMismatch,
                        node,
                        "the branches of an `if` hold values of different types",
                    );
                    return Sig::ERROR;
                };
                let Some(family) = self.one_family(node, &[(*then, second), (*otherwise, third)])
                else {
                    return Sig::ERROR;
                };
                let width = branch_width(kind, &second, &third);
                Sig::of(kind, width, reactive, family)
            }
            Op::Match { .. } => self.match_sig(node, op, operands, sigs),
            Op::Pick { items, .. } => {
                if !matches!(first.kind, Kind::Int | Kind::Any) {
                    let index = operands.first().copied().unwrap_or(node);
                    self.error_at_node(Code::IndexType, index, "an index is an integer");
                    return Sig::ERROR;
                }
                let item_sigs = sigs.get(1..).unwrap_or(&[]);
                let mut kind = item_sigs.first().map_or(Kind::Error, |sig| sig.kind);
                let mut width = item_sigs.first().map_or(Width::Flexible, |sig| sig.width);
                for sig in item_sigs.iter().skip(1) {
                    let Some(unified) = unify_kinds(kind, sig.kind) else {
                        self.error_at_node(
                            Code::TypeMismatch,
                            node,
                            "the elements of the array hold values of different types",
                        );
                        return Sig::ERROR;
                    };
                    kind = unified;
                    width = branch_width_of(kind, width, sig.width);
                }
                let pairs: Vec<(NodeId, Sig)> = items
                    .iter()
                    .copied()
                    .zip(item_sigs.iter().copied())
                    .collect();
                let Some(family) = self.one_family(node, &pairs) else {
                    return Sig::ERROR;
                };
                let width = if first.reactive {
                    Width::Adaptive
                } else {
                    width
                };
                Sig::of(kind, width, reactive, family)
            }
            Op::Convert(target, _) => {
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
                    self.error_at_node(Code::ConversionType, node, message);
                    return Sig::ERROR;
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
                first.family,
            ),
            Op::Builtin(builtin, _) => self.builtin_sig(node, *builtin, operands, sigs),
            Op::Recursive { .. } => {
                self.error_at_node(
                    Code::ReactiveRecursion,
                    node,
                    "a call to a recursive function has compile-time arguments, and this one reads a cell",
                );
                Sig::ERROR
            }
            Op::ToRead { attrs, .. } => self.destination_read_sig(node, attrs, extra),
        }
    }

    fn number_sig(&self, number: &Number, suffix: Suffix) -> Sig {
        let literal_kind = match number {
            Number::Int(_) => Kind::Int,
            Number::Float(_) => Kind::Float,
        };
        match suffix {
            Suffix::None => Sig::of(literal_kind, Width::Flexible, false, None),
            Suffix::Type(scalar) => Sig::of(kind_of(scalar), Width::Fixed(scalar), false, None),
            Suffix::Unit { family, unit } => {
                let integral = self
                    .vocabulary
                    .family(family)
                    .and_then(|decl| decl.units.get(unit))
                    .is_some_and(|unit| unit.scale.fract() == 0.0);
                let kind = if literal_kind == Kind::Int && integral {
                    Kind::Int
                } else {
                    Kind::Float
                };
                Sig::of(kind, Width::Flexible, false, Some(family))
            }
        }
    }

    fn concat_sig(&mut self, operands: &[NodeId], sigs: &[Sig]) -> Sig {
        for (operand, sig) in operands.iter().zip(sigs) {
            if sig.reactive {
                self.error_at_node(
                    Code::ReactiveString,
                    *operand,
                    "a string exists only at compile time, and this interpolation reads a cell",
                );
                return Sig::ERROR;
            }
            if matches!(sig.kind, Kind::Node | Kind::Bare) {
                let message = if sig.kind == Kind::Node {
                    "an interpolation holds a number, a `bool`, an enum value or a string, not an instance"
                } else {
                    "a variant in an interpolation is written `Enum::Variant`"
                };
                self.error_at_node(Code::InterpolationType, *operand, message);
                return Sig::ERROR;
            }
        }
        Sig::of(Kind::Str, Width::Fixed(Scalar::Str), false, None)
    }

    fn binary_sig(
        &mut self,
        node: NodeId,
        op: BinaryOp,
        lhs: (NodeId, Sig),
        rhs: (NodeId, Sig),
    ) -> Sig {
        let (a, b) = (lhs.1, rhs.1);
        let reactive = a.reactive || b.reactive;
        let numeric = a.is_numeric() && b.is_numeric();
        match op {
            BinaryOp::Add if a.kind == Kind::Str && b.kind == Kind::Str => {
                if reactive {
                    self.error_at_node(
                        Code::ReactiveString,
                        node,
                        "a string expression reads no cell",
                    );
                    return Sig::ERROR;
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
                    self.error_at_node(Code::OperandType, node, message);
                    return Sig::ERROR;
                }
                let family = match op {
                    BinaryOp::Mul => match (a.family, b.family) {
                        (Some(_), Some(_)) => {
                            self.error_at_node(
                                Code::UnitOperation,
                                node,
                                "two unit values are not multiplied",
                            );
                            return Sig::ERROR;
                        }
                        (family, None) | (None, family) => family,
                    },
                    BinaryOp::Pow => {
                        if a.family.is_some() || b.family.is_some() {
                            self.error_at_node(
                                Code::UnitOperation,
                                node,
                                "`**` takes no unit value",
                            );
                            return Sig::ERROR;
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
                    | BinaryOp::Or => match self.one_family(node, &[lhs, rhs]) {
                        Some(family) => family,
                        None => return Sig::ERROR,
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
                    self.error_at_node(Code::OperandType, node, message);
                    return Sig::ERROR;
                }
                let family = match (a.family, b.family) {
                    (Some(x), Some(y)) if x == y => None,
                    (Some(_), Some(_)) => {
                        self.error_at_node(
                            Code::MixedFamilies,
                            node,
                            "these values belong to different unit families",
                        );
                        return Sig::ERROR;
                    }
                    (Some(family), None) => Some(family),
                    (None, Some(_)) => {
                        self.error_at_node(
                            Code::UnitOperation,
                            node,
                            "a plain number is not divided by a unit value",
                        );
                        return Sig::ERROR;
                    }
                    (None, None) => None,
                };
                if op == BinaryOp::IntDiv {
                    return Sig::of(Kind::Int, arith_width(&a, &b), reactive, family);
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
                    );
                    return Sig::ERROR;
                }
                if self.one_family(node, &[lhs, rhs]).is_none() {
                    return Sig::ERROR;
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
                        _ => false,
                    };
                if !comparable {
                    self.error_at_node(
                        Code::OperandType,
                        node,
                        "`is` compares two numbers, `bool`s, values of one enum or strings",
                    );
                    return Sig::ERROR;
                }
                if (a.kind == Kind::Str || b.kind == Kind::Str) && reactive {
                    self.error_at_node(
                        Code::ReactiveString,
                        node,
                        "a string expression reads no cell",
                    );
                    return Sig::ERROR;
                }
                if numeric && self.one_family(node, &[lhs, rhs]).is_none() {
                    return Sig::ERROR;
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
            BinaryOp::And | BinaryOp::Or => {
                if !matches!(a.kind, Kind::Bool | Kind::Any)
                    || !matches!(b.kind, Kind::Bool | Kind::Any)
                {
                    self.error_at_node(Code::OperandType, node, "`and` and `or` take `bool`s");
                    return Sig::ERROR;
                }
                Sig::of(Kind::Bool, Width::Fixed(Scalar::Bool), reactive, None)
            }
        }
    }

    fn builtin_sig(
        &mut self,
        node: NodeId,
        builtin: Builtin,
        operands: &[NodeId],
        sigs: &[Sig],
    ) -> Sig {
        let reactive = sigs.iter().any(|sig| sig.reactive);
        if sigs.iter().any(|sig| !sig.is_numeric()) {
            self.error_at_node(
                Code::OperandType,
                node,
                format!("`{}` takes numbers", builtin.name()),
            );
            return Sig::ERROR;
        }
        let pairs: Vec<(NodeId, Sig)> =
            operands.iter().copied().zip(sigs.iter().copied()).collect();
        let combined =
            sigs.iter()
                .skip(1)
                .fold(sigs.first().copied().unwrap_or(Sig::ERROR), |acc, sig| {
                    Sig::of(
                        numeric_kind(acc.kind, sig.kind),
                        arith_width(&acc, sig),
                        reactive,
                        None,
                    )
                });
        match builtin {
            Builtin::Abs => sigs.first().copied().unwrap_or(Sig::ERROR),
            Builtin::Min | Builtin::Max | Builtin::Clamp => {
                let Some(family) = self.one_family(node, &pairs) else {
                    return Sig::ERROR;
                };
                Sig::of(combined.kind, combined.width, reactive, family)
            }
            Builtin::Floor | Builtin::Ceil | Builtin::Round => {
                let first = sigs.first().copied().unwrap_or(Sig::ERROR);
                Sig::of(
                    Kind::Float,
                    float_width(first.width),
                    reactive,
                    first.family,
                )
            }
            Builtin::Lerp => {
                let Some(family) = self.one_family(node, pairs.get(..2).unwrap_or(&[])) else {
                    return Sig::ERROR;
                };
                if sigs.get(2).is_some_and(|sig| sig.family.is_some()) {
                    self.error_at_node(
                        Code::UnitOperation,
                        node,
                        "the third argument of `lerp` is a plain number",
                    );
                    return Sig::ERROR;
                }
                Sig::of(Kind::Float, float_width(combined.width), reactive, family)
            }
            Builtin::Rand => {
                if sigs
                    .iter()
                    .any(|sig| !matches!(sig.kind, Kind::Int | Kind::Any))
                {
                    self.error_at_node(Code::OperandType, node, "`rand` takes two integers");
                    return Sig::ERROR;
                }
                if sigs.iter().any(|sig| sig.family.is_some()) {
                    self.error_at_node(Code::UnitOperation, node, "`rand` takes plain numbers");
                    return Sig::ERROR;
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
                    );
                    return Sig::ERROR;
                }
                Sig::of(Kind::Float, float_width(combined.width), reactive, None)
            }
        }
    }

    fn destination_read_sig(
        &mut self,
        node: NodeId,
        attrs: &[(
            crate::semantics::analysis::item_ref::ItemRef,
            symphonyscript_core::AttrId,
        )],
        extra: Extra,
    ) -> Sig {
        let reactive = matches!(extra, Extra::ToRead { reactive: true });
        let mut decls = attrs
            .iter()
            .filter_map(|(item, attr)| self.attr_decl(*item, *attr).cloned());
        let Some(first) = decls.next() else {
            return Sig::ERROR;
        };
        let family: Option<FamilyId> = first.unit.map(|unit| unit.family);
        for decl in decls {
            if decl.ty != first.ty || decl.unit.map(|unit| unit.family) != family {
                self.error_at_node(
                    Code::ToCandidates,
                    node,
                    "every destination candidate has this attribute with one type and unit family",
                );
                return Sig::ERROR;
            }
        }
        if !first.ty.is_scalar() {
            self.error_at_node(
                Code::ToCandidates,
                node,
                "`to` reads a single-valued attribute",
            );
            return Sig::ERROR;
        }
        let scalar = first.ty.scalar;
        let width = if reactive {
            Width::Adaptive
        } else {
            Width::Fixed(scalar)
        };
        Sig::of(kind_of(scalar), width, reactive, family)
    }
}

pub(super) fn branch_width(kind: Kind, a: &Sig, b: &Sig) -> Width {
    branch_width_of(kind, a.width, b.width)
}

pub(super) fn branch_width_of(kind: Kind, a: Width, b: Width) -> Width {
    match kind {
        Kind::Int | Kind::Float => {
            let left = Sig::of(kind, a, false, None);
            let right = Sig::of(kind, b, false, None);
            arith_width(&left, &right)
        }
        Kind::Bool => Width::Fixed(Scalar::Bool),
        Kind::Str => Width::Fixed(Scalar::Str),
        Kind::Enum(id) => Width::Fixed(Scalar::Enum(id)),
        Kind::Node | Kind::Bare | Kind::Any | Kind::Error => Width::Flexible,
    }
}
