use symphonyscript_core::AttrId;

use super::eval_ty::EvalTy;
use super::ids::{NodeId, ScopeId, WireId};
use super::item_ref::ItemRef;
use crate::semantics::ir::{ExprId, InputId, LetId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum LetRef {
    Top(LetId),
    Local(u32),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Query {
    Expr(ExprId, ScopeId),
    Let(ScopeId, LetRef),
    Param(ScopeId, u32),
    Input(InputId),
    Attr(ItemRef, AttrId),
    Destination(WireId),
    Sig(NodeId),
    Eval(NodeId, EvalTy),
}
