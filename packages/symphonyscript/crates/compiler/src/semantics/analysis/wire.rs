use std::rc::Rc;

use symphonyscript_core::{Segment, WireKind};

use super::attr_source::AttrSource;
use super::chain::Chain;
use super::condition::Condition;
use super::ids::{InstanceId, ScopeId};
use super::path_parent::PathParent;
use crate::semantics::ir::{Loc, WireId as HirWireId};

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Wire {
    pub kind: WireKind,
    pub wire: HirWireId,
    pub scope: ScopeId,
    pub clause_scope: ScopeId,
    pub name: Option<String>,
    pub segment: Option<Segment>,
    pub path_parent: Option<PathParent>,
    pub source: InstanceId,
    pub uses: Chain,
    pub conditions: Rc<Vec<Condition>>,
    pub attrs: Vec<AttrSource>,
    pub loc: Loc,
}
