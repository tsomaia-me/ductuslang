#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum I64Position {
    Index,
    Offset,
    SliceBound,
}
