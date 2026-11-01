#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cost {
    pub(super) work: usize,
    pub(super) peak_stored_entries: usize,
    pub(super) tokens: usize,
}
