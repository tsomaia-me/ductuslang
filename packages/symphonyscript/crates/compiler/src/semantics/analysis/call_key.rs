use std::rc::Rc;

use symphonyscript_core::{EnumId, FamilyId};

use super::eval_ty::EvalTy;
use super::item_ref::ItemRef;
use super::template_key::ArgShape;
use crate::semantics::ir::FnId;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ArgKey {
    Int(i128, EvalTy),
    F32(u32),
    F64(u64),
    Bool(bool),
    Enum(EnumId, u16),
    Str(Rc<str>),
    Node(ItemRef),
    Array(Vec<ArgKey>),
    Default,
    Error,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct CallKey {
    pub function: FnId,
    pub args: Vec<ArgKey>,
    pub shapes: Vec<ArgShape>,
    pub expected: EvalTy,
    pub units: Vec<(FamilyId, usize)>,
}
