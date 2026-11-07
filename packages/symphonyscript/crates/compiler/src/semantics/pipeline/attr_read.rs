use symphonyscript_core::Span;

use crate::semantics::operators::EvalTy;

#[derive(Clone, Debug)]
pub(crate) struct AttrRead {
    pub row: u32,
    pub attr: usize,
    pub root: u32,
    pub ty: EvalTy,
    pub label: String,
    pub span: Span,
    pub key: (u32, u32),
    pub cyclic: bool,
    pub wire: bool,
    pub set: Option<bool>,
}
