#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BinderKind {
    Cell,
    Let,
    FnParam,
    PatternParam,
    LocalLet,
    PlacementLoop,
    BundleLoop,
    ForExprLoop,
    Accumulator,
}
