use alloc::boxed::Box;
use alloc::vec::Vec;

use super::binary_op::BinaryOp;
use super::builtin::Builtin;
use super::unary_op::UnaryOp;
use crate::ids::{CellId, InputSlot};
use crate::types::{Ty, Value};

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Expr {
    Const(Value),
    Input(InputSlot),
    Cell(CellId),
    Unary {
        op: UnaryOp,
        ty: Ty,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        ty: Ty,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then: Box<Expr>,
        otherwise: Box<Expr>,
    },
    Match {
        ty: Ty,
        scrutinee: Box<Expr>,
        arms: Vec<(Value, Expr)>,
        otherwise: Option<Box<Expr>>,
    },
    Pick {
        index_ty: Ty,
        index: Box<Expr>,
        items: Vec<Expr>,
    },
    Convert {
        from: Ty,
        to: Ty,
        operand: Box<Expr>,
    },
    Call {
        builtin: Builtin,
        ty: Ty,
        args: Vec<Expr>,
    },
}
