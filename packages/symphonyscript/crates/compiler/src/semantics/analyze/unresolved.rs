use crate::semantics::ir::{ExprId, LetId};

pub(crate) struct Unresolved {
    pub names: Vec<ExprId>,
    pub computed_lets: Vec<LetId>,
}
