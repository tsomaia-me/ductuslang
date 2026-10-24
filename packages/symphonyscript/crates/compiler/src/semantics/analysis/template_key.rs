use symphonyscript_core::FamilyId;

use super::arg_shape::ArgShape;
use crate::semantics::ir::FnId;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct TemplateKey {
    pub function: FnId,
    pub args: Vec<ArgShape>,
    pub units: Vec<(FamilyId, usize)>,
}
