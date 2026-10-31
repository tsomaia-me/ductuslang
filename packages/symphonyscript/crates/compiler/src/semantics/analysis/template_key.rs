use symphonyscript_core::FamilyId;

use super::arg_shape::ArgShape;
use super::interned::Interned;
use super::template_parts::TemplateParts;
use crate::semantics::ir::FnId;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct TemplateKey {
    pub function: FnId,
    pub args: Vec<ArgShape>,
    pub units: Vec<(FamilyId, usize)>,
}

impl TemplateKey {
    pub fn parts(&self) -> TemplateParts<'_> {
        TemplateParts {
            function: self.function,
            args: &self.args,
            units: &self.units,
        }
    }
}

impl Interned for TemplateKey {
    type Parts<'a> = TemplateParts<'a>;

    fn holds(&self, parts: TemplateParts<'_>) -> bool {
        self.parts() == parts
    }

    fn owned(parts: TemplateParts<'_>) -> TemplateKey {
        crate::semantics::counted_walk::CountedWalk::TemplateKeyOwned.step();
        TemplateKey {
            function: parts.function,
            args: parts.args.to_vec(),
            units: parts.units.to_vec(),
        }
    }
}
