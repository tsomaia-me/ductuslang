use crate::semantics::analysis::chain::Chain;
use crate::semantics::analysis::ids::{ConditionsId, InstanceId};
use crate::semantics::analysis::path_parent::PathParent;

pub(crate) struct Context {
    pub owner: Option<InstanceId>,
    pub path_parent: Option<PathParent>,
    pub conditions: Option<ConditionsId>,
    pub uses: Chain,
}
