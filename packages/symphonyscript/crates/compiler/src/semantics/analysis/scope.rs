use std::collections::BTreeMap;
use std::rc::Rc;

use super::bound::BoundVal;
use super::chain::Chain;
use super::ids::{ScopeId, WireId};
use super::item_ref::ItemRef;
use super::val::Val;
use crate::semantics::ir::{ExprId, FnId, PatternId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LocalsOwner {
    Fn(FnId),
    Pattern(PatternId),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LocalName {
    Param(u32),
    Let(u32),
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum ParamSource {
    Arg { expr: ExprId, scope: ScopeId },
    Default,
    Missing,
    Bound(BoundVal),
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Locals {
    pub owner: LocalsOwner,
    pub params: Vec<ParamSource>,
    pub names: Rc<BTreeMap<String, LocalName>>,
    pub recursive: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum ScopeKind {
    Global { prelude: bool },
    Locals(Locals),
    ParamPrefix { locals: ScopeId, count: u32 },
    Body { names: BTreeMap<String, ItemRef> },
    ForVar { name: String, value: Val },
    WireClause { wire: WireId },
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Scope {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub chain: Chain,
    pub template: Option<ScopeId>,
}
