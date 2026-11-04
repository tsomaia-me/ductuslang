use super::reason::Reason;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Declined(pub Reason);
