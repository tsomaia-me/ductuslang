#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BoundForm {
    Absent,
    Reactive,
    Start(i128),
    End(i128),
}
