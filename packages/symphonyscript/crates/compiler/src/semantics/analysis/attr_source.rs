use symphonyscript_core::AttrId;

use super::ids::ScopeId;
use crate::semantics::ir::{ExprId, Loc};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AttrValueSource {
    Expr(ExprId, ScopeId),
    Set(bool),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct AttrSource {
    pub attr: AttrId,
    pub value: AttrValueSource,
    pub loc: Loc,
}
