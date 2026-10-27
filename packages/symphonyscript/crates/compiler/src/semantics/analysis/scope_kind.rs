use std::collections::BTreeMap;

use super::ids::{ScopeId, WireId};
use super::item_ref::ItemRef;
use super::locals::Locals;
use super::val::Val;
use crate::semantics::ir::WireId as HirWireId;

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum ScopeKind {
    Global { prelude: bool },
    Locals(Locals),
    ParamPrefix { locals: ScopeId, count: u32 },
    Body { names: BTreeMap<String, ItemRef> },
    ForVar { name: String, value: Val },
    WireClause { wire: WireId },
    WrittenClause { wire: HirWireId },
}
