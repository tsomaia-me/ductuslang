use alloc::vec;
use alloc::vec::Vec;

use super::binary_op::BinaryOp;
use super::builtin::Builtin;
use super::expr_ref::ExprRef;
use super::unary_op::UnaryOp;
use crate::ids::{CellId, InputSlot};
use crate::types::{Ty, Value};

#[derive(Clone, Debug)]
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

    pub(crate) fn same_shape(&self, other: &ExprNode) -> bool {
        match (self, other) {
            (ExprNode::Const(a), ExprNode::Const(b)) => a == b,
            (ExprNode::Input(a), ExprNode::Input(b)) => a == b,
            (ExprNode::Cell(a), ExprNode::Cell(b)) => a == b,
            (
                ExprNode::Unary { op, ty, .. },
                ExprNode::Unary {
                    op: other_op,
                    ty: other_ty,
                    ..
                },
            ) => op == other_op && ty == other_ty,
            (
                ExprNode::Binary { op, ty, .. },
                ExprNode::Binary {
                    op: other_op,
                    ty: other_ty,
                    ..
                },
            ) => op == other_op && ty == other_ty,
            (ExprNode::If { .. }, ExprNode::If { .. }) => true,
            (
                ExprNode::Match {
                    ty,
                    arms,
                    otherwise,
                    ..
                },
                ExprNode::Match {
                    ty: other_ty,
                    arms: other_arms,
                    otherwise: other_otherwise,
                    ..
                },
            ) => {
                ty == other_ty
                    && otherwise.is_some() == other_otherwise.is_some()
                    && arms.len() == other_arms.len()
                    && arms
                        .iter()
                        .zip(other_arms)
                        .all(|((label, _), (other_label, _))| label == other_label)
            }
            (
                ExprNode::Pick {
                    index_ty, items, ..
                },
                ExprNode::Pick {
                    index_ty: other_ty,
                    items: other_items,
                    ..
                },
            ) => index_ty == other_ty && items.len() == other_items.len(),
            (
                ExprNode::Convert { from, to, .. },
                ExprNode::Convert {
                    from: other_from,
                    to: other_to,
                    ..
                },
            ) => from == other_from && to == other_to,
            (
                ExprNode::Call { builtin, ty, args },
                ExprNode::Call {
                    builtin: other_builtin,
                    ty: other_ty,
                    args: other_args,
                },
            ) => builtin == other_builtin && ty == other_ty && args.len() == other_args.len(),
            _ => false,
        }
    }

    pub(crate) fn hash_shape<H: core::hash::Hasher>(&self, state: &mut H) {
        use core::hash::Hash;
        core::mem::discriminant(self).hash(state);
        match self {
            ExprNode::Const(value) => value.hash(state),
            ExprNode::Input(slot) => slot.hash(state),
            ExprNode::Cell(cell) => cell.hash(state),
            ExprNode::Unary { op, ty, .. } => {
                op.hash(state);
                ty.hash(state);
            }
            ExprNode::Binary { op, ty, .. } => {
                op.hash(state);
                ty.hash(state);
            }
            ExprNode::If { .. } => {}
            ExprNode::Match {
                ty,
                arms,
                otherwise,
                ..
            } => {
                ty.hash(state);
                otherwise.is_some().hash(state);
                arms.len().hash(state);
                for (label, _) in arms {
                    label.hash(state);
                }
            }
            ExprNode::Pick {
                index_ty, items, ..
            } => {
                index_ty.hash(state);
                items.len().hash(state);
            }
            ExprNode::Convert { from, to, .. } => {
                from.hash(state);
                to.hash(state);
            }
            ExprNode::Call { builtin, ty, args } => {
                builtin.hash(state);
                ty.hash(state);
                args.len().hash(state);
            }
        }
    }
}
