#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Extreme {
    Unbounded,
    Start(i128),
    End(i128),
}
