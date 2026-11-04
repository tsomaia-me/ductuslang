use super::convert::convert;
use super::eval_ty::EvalTy;
use super::operator_host::{Held, OperatorHost};
use super::represents::represents;
use super::ty_name::ty_name;
use super::value::Value;
use crate::diagnostics::Code;
use crate::semantics::Internal;

pub(crate) fn converted_integer<H: OperatorHost + ?Sized>(
    host: &mut H,
    node: H::Node,
    flexible: bool,
    value: Held<H>,
    ty: EvalTy,
) -> Result<Held<H>, Internal> {
    let Value::Int(integer) = value else {
        return Ok(value);
    };
    let converted = convert(value, ty);
    let exactly = match converted {
        Value::F32(float) => represents(integer, f64::from(float)),
        Value::F64(float) => represents(integer, float),
        Value::Int(_)
        | Value::Bool(_)
        | Value::Enum(..)
        | Value::Str(_)
        | Value::Node(_)
        | Value::Array(_)
        | Value::Error => true,
    };
    if flexible && !exactly {
        host.fault(
            Code::DoesNotFit,
            node,
            format!(
                "the integer {integer} converts to `{}` inexactly",
                ty_name(ty)
            ),
        )?;
        return Ok(Value::Error);
    }
    Ok(converted)
}
