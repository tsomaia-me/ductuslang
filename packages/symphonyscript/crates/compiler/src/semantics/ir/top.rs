use super::ids::{FnId, InputId, LetId, PatternId, PlacementId, StateId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Top {
    Input(InputId),
    Let(LetId),
    State(StateId),
    Fn(FnId),
    Pattern(PatternId),
    Placement(PlacementId),
    Placeholder(u32),
}
