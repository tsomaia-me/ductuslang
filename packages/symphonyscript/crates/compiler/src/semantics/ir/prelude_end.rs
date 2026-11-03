#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PreludeEnd {
    pub top: usize,
    pub files: usize,
    pub fns: usize,
    pub patterns: usize,
    pub lets: usize,
}
