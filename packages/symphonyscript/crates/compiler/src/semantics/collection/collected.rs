use super::cell_groups::CellGroups;
use super::inline_counts::InlineCounts;
use super::namespace::Namespace;
use super::stand_in::StandIn;
use crate::semantics::ir::ExprId;

#[derive(Clone, Debug, Default)]
pub(crate) struct Collected {
    pub source: Namespace,
    pub prelude: Namespace,
    pub recursive: Vec<bool>,
    pub calls: Vec<Vec<(usize, ExprId)>>,
    pub inline: InlineCounts,
    pub cyclic_patterns: Vec<bool>,
    pub stand_ins: Vec<Option<StandIn>>,
    pub placeholder_patterns: Vec<bool>,
    pub cell_groups: CellGroups,
}
