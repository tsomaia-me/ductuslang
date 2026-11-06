use super::memo_arg::MemoArg;
use crate::semantics::operators::EvalTy;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct MemoKey {
    pub body: u32,
    pub ty: EvalTy,
    pub flags: [bool; 3],
    pub args: Vec<MemoArg>,
    pub unit: Option<(symphonyscript_core::FamilyId, usize, u32)>,
}
