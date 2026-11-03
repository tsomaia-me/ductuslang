use super::operator_host::OperatorHost;
use crate::diagnostics::{Code, index_outside_help};
use crate::semantics::Internal;

pub(crate) fn index_outside<H: OperatorHost + ?Sized>(
    host: &mut H,
    index: H::Node,
    position: i128,
    length: u64,
) -> Result<(), Internal> {
    let message = format!("the index {position} is outside the array of length {length}");
    match index_outside_help(length) {
        Some(help) => host.fault_helped(Code::IndexOutOfRange, index, message, help),
        None => host.fault(Code::IndexOutOfRange, index, message),
    }
}
