#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Bracket {
    Expression,
    Bundle { depth: u32, header: bool },
    Other,
}
