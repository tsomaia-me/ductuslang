#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Settling {
    Settled,
    Pending,
    Blocked,
}
