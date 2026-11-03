use std::rc::Rc;

use symphonyscript_core::{BinaryOp, SpecFloat};

use super::compare::compare;
use super::convert::convert;
use super::converted_integer::converted_integer;
use super::eval_ty::EvalTy;
use super::exact_binary::exact_binary;
use super::float_binary::float_binary;
use super::i64_position::I64Position;
use super::index_outside::index_outside;
use super::int_position::int_position;
use super::integer::integer;
use super::messages::{JOIN_SINGLE, NOT_AN_ARRAY, RAGGED};
use super::operands::Operands;
use super::operator::Operator;
use super::operator_host::{Held, OperatorHost};
use super::outside_i64::outside_i64;
use super::value::Value;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::builtin::Builtin;

fn missed() -> Internal {
    Internal::missed("operator operands")
}

fn one<V>(operands: Operands<V>) -> Result<V, Internal> {
    let Operands::One(value) = operands else {
        return Err(missed());
    };
    Ok(value)
}

fn two<V>(operands: Operands<V>) -> Result<(V, V), Internal> {
    let Operands::Two(a, b) = operands else {
        return Err(missed());
    };
    Ok((a, b))
}

fn many<V>(operands: Operands<V>) -> Result<Vec<V>, Internal> {
    let Operands::Many(values) = operands else {
        return Err(missed());
    };
    Ok(values)
}

pub(crate) fn plan_op<H: OperatorHost + ?Sized>(
    host: &mut H,
    operator: Operator<H::Node>,
    operands: Operands<Held<H>>,
) -> Result<Held<H>, Internal> {
    Ok(match operator {
        Operator::Not => match one(operands)? {
            Value::Bool(value) => Value::Bool(!value),
            Value::Int(_)
            | Value::F32(_)
            | Value::F64(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Array(_)
            | Value::Error => Value::Error,
        },
        Operator::Neg(node, own) => match one(operands)? {
            Value::Int(value) => integer(host, node, own, value.checked_neg())?,
            Value::F32(value) => Value::F32(-value),
            Value::F64(value) => Value::F64(-value),
            Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Array(_)
            | Value::Error => Value::Error,
        },
        Operator::Logic(op) => match two(operands)? {
            (Value::Bool(a), Value::Bool(b)) => {
                Value::Bool(if op == BinaryOp::And { a && b } else { a || b })
            }
            _ => Value::Error,
        },
        Operator::Arith(op, node, own) => match two(operands)? {
            (Value::Int(a), Value::Int(b)) => integer(host, node, own, exact_binary(op, a, b))?,
            (Value::F32(a), Value::F32(b)) => {
                Value::F32(float_binary(op, a, b).ok_or(Internal::missed("float operator"))?)
            }
            (Value::F64(a), Value::F64(b)) => {
                Value::F64(float_binary(op, a, b).ok_or(Internal::missed("float operator"))?)
            }
            _ => Value::Error,
        },
        Operator::Compare(op) => {
            let (a, b) = two(operands)?;
            if a.is_error() || b.is_error() {
                Value::Error
            } else {
                compare(op, &a, &b)
            }
        }
        Operator::StrAdd(node) => {
            let value = match two(operands)? {
                (Value::Str(a), Value::Str(b)) => Value::Str(Rc::from(format!("{a}{b}").as_str())),
                _ => Value::Error,
            };
            host.within_value_limit(node, value, "this join")?
        }
        Operator::IntDivFloat(node, own, flexible) => match two(operands)? {
            (Value::Int(a), Value::Int(b)) => {
                let value = integer(
                    host,
                    node,
                    EvalTy::Exact,
                    exact_binary(BinaryOp::IntDiv, a, b),
                )?;
                converted_integer(host, node, flexible, value, own)?
            }
            _ => Value::Error,
        },
        Operator::Div => match two(operands)? {
            (Value::F32(a), Value::F32(b)) => Value::F32(a / b),
            (Value::F64(a), Value::F64(b)) => Value::F64(a / b),
            _ => Value::Error,
        },
        Operator::ConvertTo(own) => convert(one(operands)?, own),
        Operator::Join(node) => match two(operands)? {
            (Value::Array(left), Value::Array(right)) => joined(host, node, &left, &right)?,
            (Value::Error, _) | (_, Value::Error) => Value::Error,
            _ => {
                host.fault(Code::ArrayOperand, node, JOIN_SINGLE.to_string())?;
                Value::Error
            }
        },
        Operator::Slice(base, lower, upper) => {
            let mut values = many(operands)?.into_iter();
            let value = values.next().ok_or(Internal::missed("slice base"))?;
            let start = match lower {
                Some((bound, from_end)) => {
                    let at = values.next().ok_or(Internal::missed("slice bound"))?;
                    bound_position(host, bound, &at, from_end)?
                }
                None => None,
            };
            let end = match upper {
                Some((bound, from_end)) => {
                    let at = values.next().ok_or(Internal::missed("slice bound"))?;
                    bound_position(host, bound, &at, from_end)?
                }
                None => None,
            };
            sliced(host, base, value, start, end)?
        }
        Operator::Element(base, index) => {
            let (value, position) = two(operands)?;
            match int_position(&position) {
                Some(position) => element(host, (base, index), value, position)?,
                None => Value::Error,
            }
        }
        Operator::Array(node) => array_value(host, node, many(operands)?)?,
        Operator::Length(operand) => match one(operands)? {
            Value::Array(array) => {
                Value::Int(i128::try_from(host.length(&array)).unwrap_or(i128::MAX))
            }
            Value::Error => Value::Error,
            Value::Int(_)
            | Value::F32(_)
            | Value::F64(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_) => {
                host.fault(Code::NotAnArray, operand, NOT_AN_ARRAY.to_string())?;
                Value::Error
            }
        },
        Operator::Builtin(node, builtin, own) => {
            builtin_value(host, (node, builtin, own), many(operands)?)?
        }
    })
}

fn joined<H: OperatorHost + ?Sized>(
    host: &mut H,
    node: H::Node,
    left: &H::Array,
    right: &H::Array,
) -> Result<Held<H>, Internal> {
    match host.join(left, right) {
        Ok(array) => host.within_value_limit(node, Value::Array(array), "this join"),
        Err(Code::LengthLimit) => {
            let total = u128::try_from(host.length(left))
                .unwrap_or(u128::MAX)
                .saturating_add(u128::try_from(host.length(right)).unwrap_or(u128::MAX));
            host.fault(
                Code::LengthLimit,
                node,
                format!(
                    "this join holds {total} elements, and an array's length and capacity are below 2³¹"
                ),
            )?;
            Ok(Value::Error)
        }
        Err(code) => {
            host.fault(
                code,
                node,
                "the rows of the joined arrays have different shapes".to_string(),
            )?;
            Ok(Value::Error)
        }
    }
}

fn bound_position<H: OperatorHost + ?Sized>(
    host: &mut H,
    node: H::Node,
    value: &Held<H>,
    from_end: bool,
) -> Result<Option<(i128, bool)>, Internal> {
    Ok(match value {
        Value::Int(value) if i64::try_from(*value).is_err() => {
            host.fault(
                Code::IndexOutOfRange,
                node,
                outside_i64(I64Position::SliceBound, *value),
            )?;
            None
        }
        Value::Int(value) => Some((*value, from_end)),
        Value::F32(_)
        | Value::F64(_)
        | Value::Bool(_)
        | Value::Enum(..)
        | Value::Str(_)
        | Value::Node(_)
        | Value::Array(_)
        | Value::Error => None,
    })
}

fn sliced<H: OperatorHost + ?Sized>(
    host: &mut H,
    base: H::Node,
    value: Held<H>,
    start: Option<(i128, bool)>,
    end: Option<(i128, bool)>,
) -> Result<Held<H>, Internal> {
    let array = match value {
        Value::Array(array) => array,
        Value::Error => return Ok(Value::Error),
        Value::Int(_)
        | Value::F32(_)
        | Value::F64(_)
        | Value::Bool(_)
        | Value::Enum(..)
        | Value::Str(_)
        | Value::Node(_) => {
            host.fault(
                Code::NotAnArray,
                base,
                "only an array is sliced".to_string(),
            )?;
            return Ok(Value::Error);
        }
    };
    let length = i128::try_from(host.length(&array)).unwrap_or(i128::MAX);
    let resolve = |bound: Option<(i128, bool)>, absent: i128| match bound {
        None => Some(absent),
        Some((value, from_end)) => {
            let position = if from_end {
                length.checked_sub(value)?
            } else {
                value
            };
            Some(position.clamp(0, length))
        }
    };
    let (Some(start), Some(end)) = (resolve(start, 0), resolve(end, length)) else {
        return Ok(Value::Error);
    };
    let end = end.max(start);
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return Ok(Value::Error);
    };
    Ok(Value::Array(host.slice(&array, start, end)))
}

fn element<H: OperatorHost + ?Sized>(
    host: &mut H,
    (base, index): (H::Node, H::Node),
    value: Held<H>,
    position: i128,
) -> Result<Held<H>, Internal> {
    let array = match value {
        Value::Array(array) => array,
        Value::Error => return Ok(Value::Error),
        Value::Int(_)
        | Value::F32(_)
        | Value::F64(_)
        | Value::Bool(_)
        | Value::Enum(..)
        | Value::Str(_)
        | Value::Node(_) => {
            host.fault(
                Code::NotAnArray,
                base,
                "only an array is indexed".to_string(),
            )?;
            return Ok(Value::Error);
        }
    };
    let item = usize::try_from(position)
        .ok()
        .and_then(|position| host.element(&array, position));
    Ok(match item {
        Some(item) => item,
        None => {
            let length = u64::try_from(host.length(&array)).unwrap_or(u64::MAX);
            index_outside(host, index, position, length)?;
            Value::Error
        }
    })
}

fn array_value<H: OperatorHost + ?Sized>(
    host: &mut H,
    node: H::Node,
    values: Vec<Held<H>>,
) -> Result<Held<H>, Internal> {
    if values.iter().any(Value::is_error) {
        return Ok(Value::Error);
    }
    let Some(first) = values.first() else {
        return Ok(Value::Array(host.array(Vec::new())));
    };
    let inner = host.lengths(first);
    let mut ragged = false;
    for value in &values {
        ragged = ragged || host.lengths(value) != inner;
    }
    if ragged {
        host.fault(Code::RaggedArray, node, RAGGED.to_string())?;
        return Ok(Value::Error);
    }
    let array = host.array(values);
    host.within_value_limit(node, Value::Array(array), "this array")
}

fn builtin_value<H: OperatorHost + ?Sized>(
    host: &mut H,
    (node, builtin, own): (H::Node, Builtin, EvalTy),
    values: Vec<Held<H>>,
) -> Result<Held<H>, Internal> {
    let ints: Option<Vec<i128>> = values
        .iter()
        .map(|value| match value {
            Value::Int(value) => Some(*value),
            Value::F32(_)
            | Value::F64(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Array(_)
            | Value::Error => None,
        })
        .collect();
    if let Some(ints) = ints {
        let value = match (builtin, ints.as_slice()) {
            (Builtin::Abs, [x]) => x.checked_abs(),
            (Builtin::Min, [a, b]) => Some(*a.min(b)),
            (Builtin::Max, [a, b]) => Some(*a.max(b)),
            (Builtin::Clamp, [x, lo, hi]) => Some(*x.max(lo).min(hi)),
            _ => return Ok(Value::Error),
        };
        return integer(host, node, own, value);
    }
    let floats32: Option<Vec<f32>> = values
        .iter()
        .map(|value| match value {
            Value::F32(value) => Some(*value),
            Value::Int(_)
            | Value::F64(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Array(_)
            | Value::Error => None,
        })
        .collect();
    if let Some(floats) = floats32 {
        return Ok(Value::F32(
            builtin
                .core()
                .and_then(|core| f32::spec_builtin(core, &floats))
                .ok_or(Internal::missed("built-in"))?,
        ));
    }
    let floats64: Option<Vec<f64>> = values
        .iter()
        .map(|value| match value {
            Value::F64(value) => Some(*value),
            Value::Int(_)
            | Value::F32(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Array(_)
            | Value::Error => None,
        })
        .collect();
    Ok(match floats64 {
        Some(floats) => Value::F64(
            builtin
                .core()
                .and_then(|core| f64::spec_builtin(core, &floats))
                .ok_or(Internal::missed("built-in"))?,
        ),
        None => Value::Error,
    })
}
