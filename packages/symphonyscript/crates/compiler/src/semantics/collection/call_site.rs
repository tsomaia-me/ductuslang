use crate::semantics::ir::FnId;

#[derive(Clone, Debug)]
pub(crate) struct CallSite {
    pub callee: FnId,
    pub defaulted: Vec<usize>,
    pub recursive: bool,
}
