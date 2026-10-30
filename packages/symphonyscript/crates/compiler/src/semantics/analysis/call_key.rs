use symphonyscript_core::FamilyId;

use super::arg_key::ArgKey;
use super::arg_shape::ArgShape;
use super::eval_ty::EvalTy;
use crate::semantics::ir::FnId;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct CallKey {
    pub function: FnId,
    pub args: Vec<ArgKey>,
    pub shapes: Vec<ArgShape>,
    pub expected: EvalTy,
    pub units: Vec<(FamilyId, usize)>,
}
