#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Iteration {
    pub names: (u32, u32),
    pub range: bool,
    pub iterable: (u32, u32),
}
