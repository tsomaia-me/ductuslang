use super::arg_key::ArgKey;
use super::eval_ty::EvalTy;
use super::ids::TemplateId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct CallParts<'a> {
    pub template: TemplateId,
    pub args: &'a [ArgKey],
    pub expected: EvalTy,
}
