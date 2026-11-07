#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Slice {
    One,
    Two,
    Three,
    Four,
    Five,
    Flip,
    DefaultFlip,
    Pending,
}

pub(crate) const LANDED: Slice = Slice::Flip;
