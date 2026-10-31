#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum CellWord {
    State,
    Stream,
    Channel,
}
