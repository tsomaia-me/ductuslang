use super::ids::NodeId;
use crate::diagnostics::Code;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ShapeFault {
    pub node: NodeId,
    pub code: Code,
    pub message: &'static str,
}
