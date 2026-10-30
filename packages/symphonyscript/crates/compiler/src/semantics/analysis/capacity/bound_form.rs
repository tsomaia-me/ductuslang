use super::extreme::Extreme;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BoundForm {
    Absent,
    Reactive(Extreme),
    Start(i128),
    End(i128),
}
