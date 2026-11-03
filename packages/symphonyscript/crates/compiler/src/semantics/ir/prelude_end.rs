#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct PreludeEnd {
    pub top: usize,
    pub files: usize,
    pub fns: usize,
    pub patterns: usize,
    pub lets: usize,
}
