use symphonyscript_core::AttrId;

use super::eval_ty::EvalTy;
use super::ids::{NodeId, ScopeId, WireId};
use super::item_ref::ItemRef;
use super::let_ref::LetRef;
use crate::semantics::ir::{ExprId, InputId, StateId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Query {
    Expr(ExprId, ScopeId),
    Let(ScopeId, LetRef),
    Param(ScopeId, u32),
    Input(InputId),
    State(StateId),
    Attr(ItemRef, AttrId),
    Destination(WireId),
    Sig(NodeId),
    Eval(NodeId, EvalTy),
}
