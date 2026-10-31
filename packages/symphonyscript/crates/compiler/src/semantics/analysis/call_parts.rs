use symphonyscript_core::FamilyId;

use super::arg_key::ArgKey;
use super::arg_shape::ArgShape;
use super::call_key::CallKey;
use super::eval_ty::EvalTy;
use crate::semantics::ir::FnId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct CallParts<'a> {
    pub function: FnId,
    pub args: &'a [ArgKey],
    pub shapes: &'a [ArgShape],
    pub expected: EvalTy,
    pub units: &'a [(FamilyId, usize)],
}

impl CallParts<'_> {
    pub fn owned(&self) -> CallKey {
        CallKey {
            function: self.function,
            args: self.args.to_vec(),
            shapes: self.shapes.to_vec(),
            expected: self.expected,
            units: self.units.to_vec(),
        }
    }
}
