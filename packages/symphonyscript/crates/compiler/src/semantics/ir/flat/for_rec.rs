#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ForRec {
    pub names: (u32, u32),
    pub range: bool,
    pub iterable: (u32, u32),
    pub accumulator: u32,
    pub start: u32,
    pub lets: (u32, u32),
    pub body: u32,
}
