use symphonyscript_core::FamilyId;

use super::sig::{Kind, Width};
use crate::semantics::ir::FnId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ArgSig {
    pub kind: Kind,
    pub width: Width,
    pub family: Option<FamilyId>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ArgShape {
    Default,
    Scalar(ArgSig),
    Array(Vec<u32>, Vec<ArgSig>),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct TemplateKey {
    pub function: FnId,
    pub args: Vec<ArgShape>,
    pub units: Vec<(FamilyId, usize)>,
}
