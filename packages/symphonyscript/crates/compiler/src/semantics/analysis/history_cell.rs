use crate::semantics::ir::{InputId, LetId, StateId};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum HistoryCell {
    Input(InputId),
    Let(LetId),
    State(StateId),
}
