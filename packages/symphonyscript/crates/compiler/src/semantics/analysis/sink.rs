use super::reachable::Reachable;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Sink {
    Reached,
    Into(Reachable),
    Discard,
}
