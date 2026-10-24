use super::template_key::ArgShape;
use crate::semantics::ir::FnId;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct FamilyKey {
    pub function: FnId,
    pub args: Vec<ArgShape>,
}
