use std::rc::Rc;

use symphonyscript_core::{NodeKind, Segment};

use super::attr_source::AttrSource;
use super::chain::Chain;
use super::condition::Condition;
use super::entry::Entry;
use super::ids::{InstanceId, ScopeId};
use super::path_parent::PathParent;
use crate::semantics::ir::{Loc, PlacementId};

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Instance {
    pub kind: NodeKind,
    pub placement: PlacementId,
    pub scope: ScopeId,
    pub name: Option<String>,
    pub segment: Option<Segment>,
    pub path_parent: Option<PathParent>,
    pub owner: Option<InstanceId>,
    pub uses: Chain,
    pub conditions: Rc<Vec<Condition>>,
    pub attrs: Vec<AttrSource>,
    pub sequence: Vec<Entry>,
    pub body_scope: Option<ScopeId>,
    pub loc: Loc,
    pub expanded: bool,
}
