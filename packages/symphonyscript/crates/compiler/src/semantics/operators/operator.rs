use symphonyscript_core::BinaryOp;

use super::eval_ty::EvalTy;
use crate::semantics::builtin::Builtin;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operator<N> {
    Not,
    Neg(N, EvalTy),
    Logic(BinaryOp),
    Arith(BinaryOp, N, EvalTy),
    Compare(BinaryOp),
    StrAdd(N),
    IntDivFloat(N, EvalTy, bool),
    Div,
    ConvertTo(EvalTy),
    Join(N),
    Slice(N, Option<(N, bool)>, Option<(N, bool)>),
    Element(N, N),
    Array(N),
    Length(N),
    Builtin(N, Builtin, EvalTy),
}
