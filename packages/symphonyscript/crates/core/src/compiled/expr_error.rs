use core::fmt;

use super::expr_ref::ExprRef;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExprError {
    ForwardOperand { node: ExprRef, operand: ExprRef },
    TooManyNodes,
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExprError::ForwardOperand { node, operand } => write!(
                f,
                "node {} reads operand {}, which does not come before it",
                node.0, operand.0
            ),
            ExprError::TooManyNodes => write!(f, "the expression has more nodes than u32 indexes"),
        }
    }
}

impl core::error::Error for ExprError {}
