use super::ids::{FnId, InputId, LetId, PatternId, PlacementId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Top {
    Input(InputId),
    Let(LetId),
    Fn(FnId),
    Pattern(PatternId),
    Placement(PlacementId),
}
