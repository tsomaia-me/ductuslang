use super::family_arg::FamilyArg;
use crate::semantics::ir::FnId;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct FamilyKey {
    pub function: FnId,
    pub args: Vec<FamilyArg>,
}
