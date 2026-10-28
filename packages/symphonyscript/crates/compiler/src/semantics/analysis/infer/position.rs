use symphonyscript_core::{AttrUnit, Ty};

use super::role::Role;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, Loc};

#[derive(Clone, Debug)]
pub(crate) struct Position {
    pub val: Val,
    pub ty: Ty,
    pub unit: Option<AttrUnit>,
    pub role: Role,
    pub name: Option<String>,
    pub loc: Loc,
    pub scope: ScopeId,
    pub expr: ExprId,
}
