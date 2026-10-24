use alloc::vec;
use alloc::vec::Vec;

use super::binary_op::BinaryOp;
use super::builtin::Builtin;
use super::expr_ref::ExprRef;
use super::unary_op::UnaryOp;
use crate::ids::{CellId, InputSlot};
use crate::types::{Ty, Value};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ExprNode {
    Const(Value),
    Input(InputSlot),
    Cell(CellId),
    Unary {
        op: UnaryOp,
        ty: Ty,
        operand: ExprRef,
    },
    Binary {
        op: BinaryOp,
        ty: Ty,
        lhs: ExprRef,
        rhs: ExprRef,
    },
    If {
        condition: ExprRef,
        then: ExprRef,
        otherwise: ExprRef,
    },
    Match {
        ty: Ty,
        scrutinee: ExprRef,
        arms: Vec<(Value, ExprRef)>,
        otherwise: Option<ExprRef>,
    },
    Pick {
        index_ty: Ty,
        index: ExprRef,
        items: Vec<ExprRef>,
    },
    Convert {
        from: Ty,
        to: Ty,
        operand: ExprRef,
    },
    Call {
        builtin: Builtin,
        ty: Ty,
        args: Vec<ExprRef>,
    },
}

impl ExprNode {
    pub fn children(&self) -> Vec<ExprRef> {
        match self {
            ExprNode::Const(_) | ExprNode::Input(_) | ExprNode::Cell(_) => Vec::new(),
            ExprNode::Unary { operand, .. } | ExprNode::Convert { operand, .. } => vec![*operand],
            ExprNode::Binary { lhs, rhs, .. } => vec![*lhs, *rhs],
            ExprNode::If {
                condition,
                then,
                otherwise,
            } => vec![*condition, *then, *otherwise],
            ExprNode::Match {
                scrutinee,
                arms,
                otherwise,
                ..
            } => core::iter::once(*scrutinee)
                .chain(arms.iter().map(|(_, arm)| *arm))
                .chain(*otherwise)
                .collect(),
            ExprNode::Pick { index, items, .. } => core::iter::once(*index)
                .chain(items.iter().copied())
                .collect(),
            ExprNode::Call { args, .. } => args.clone(),
        }
    }
}
