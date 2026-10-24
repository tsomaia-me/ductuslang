#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NodeKind {
    index: u32,
}

impl NodeKind {
    pub fn index(self) -> usize {
        self.index as usize
    }
}
