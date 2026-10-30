use crate::semantics::ir::{ExprId, FnId, PatternId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LocalsOwner {
    Fn(FnId),
    Pattern(PatternId),
    For(ExprId),
}
