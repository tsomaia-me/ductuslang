use symphonyscript_core::Segment;

use super::chain::Chain;
use super::ids::{ConditionsId, InstanceId, ScopeId};
use super::path_parent::PathParent;
use crate::semantics::ir::{Loc, PatternId, PlacementId};

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct PatternUse {
    pub pattern: PatternId,
    pub placement: PlacementId,
    pub caller: ScopeId,
    pub locals: ScopeId,
    pub body: Option<ScopeId>,
    pub name: Option<String>,
    pub segment: Option<Segment>,
    pub path_parent: Option<PathParent>,
    pub owner: InstanceId,
    pub uses: Chain,
    pub conditions: Option<ConditionsId>,
    pub loc: Loc,
}
