#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct FileRec {
    pub path: (u32, u32),
    pub top: (u32, u32),
    pub regions: (u32, u32),
    pub names: (u32, u32),
    pub nodes: (u32, u32),
}
