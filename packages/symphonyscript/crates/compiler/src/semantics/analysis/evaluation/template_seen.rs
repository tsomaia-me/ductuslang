use symphonyscript_core::FamilyId;

use crate::semantics::analysis::arg_shape::ArgShape;
use crate::semantics::analysis::ids::TemplateId;

#[derive(Debug)]
pub(crate) struct TemplateSeen {
    pub shapes: Vec<ArgShape>,
    pub units: Vec<(FamilyId, usize)>,
    pub template: TemplateId,
}
