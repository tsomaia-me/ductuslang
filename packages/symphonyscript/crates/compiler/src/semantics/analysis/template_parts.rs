use symphonyscript_core::FamilyId;

use super::arg_shape::ArgShape;
use crate::semantics::ir::FnId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct TemplateParts<'a> {
    pub function: FnId,
    pub args: &'a [ArgShape],
    pub units: &'a [(FamilyId, usize)],
}
