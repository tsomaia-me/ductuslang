#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Checkpoint {
    pub(super) token: u32,
    pub(super) node: u32,
    pub(super) serial: u32,
}
