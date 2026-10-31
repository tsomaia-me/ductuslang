use super::arg_key::ArgKey;
use super::call_parts::CallParts;
use super::eval_ty::EvalTy;
use super::ids::TemplateId;
use super::interned::Interned;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct CallKey {
    pub template: TemplateId,
    pub args: Vec<ArgKey>,
    pub expected: EvalTy,
}

impl CallKey {
    pub fn parts(&self) -> CallParts<'_> {
        CallParts {
            template: self.template,
            args: &self.args,
            expected: self.expected,
        }
    }
}

impl Interned for CallKey {
    type Parts<'a> = CallParts<'a>;

    fn holds(&self, parts: CallParts<'_>) -> bool {
        self.parts() == parts
    }

    fn owned(parts: CallParts<'_>) -> CallKey {
        CallKey {
            template: parts.template,
            args: parts.args.to_vec(),
            expected: parts.expected,
        }
    }
}
