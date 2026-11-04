use super::eval_ty::EvalTy;
use super::int_range::int_range;
use super::operator_host::{Held, OperatorHost};
use super::ty_name::ty_name;
use super::value::Value;
use crate::diagnostics::Code;
use crate::semantics::Internal;

pub(crate) fn integer<H: OperatorHost + ?Sized>(
    host: &mut H,
    node: H::Node,
    own: EvalTy,
    value: Option<i128>,
) -> Result<Held<H>, Internal> {
    let Some(value) = value.filter(|value| *value != i128::MIN) else {
        host.fault(
            Code::IntegerOverflow,
            node,
            "this value's magnitude reaches 2¹²⁷".to_string(),
        )?;
        return Ok(Value::Error);
    };
    if let Some(range) = int_range(own)
        && !range.contains(&value)
    {
        host.fault(
            Code::ConstantOverflow,
            node,
            format!("this `{}` operation saturates", ty_name(own)),
        )?;
        return Ok(Value::Error);
    }
    Ok(Value::Int(value))
}
