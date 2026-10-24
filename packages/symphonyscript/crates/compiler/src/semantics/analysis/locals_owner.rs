use crate::semantics::ir::{FnId, PatternId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LocalsOwner {
    Fn(FnId),
    Pattern(PatternId),
}
