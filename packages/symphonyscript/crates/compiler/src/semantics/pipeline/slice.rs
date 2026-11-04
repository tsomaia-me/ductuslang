#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Slice {
    One,
    Two,
    Three,
    Four,
    Five,
    DefaultFlip,
    Pending,
}

pub(crate) const LANDED: Slice = Slice::Two;
