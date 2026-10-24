#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WireKind {
    index: u32,
}

impl WireKind {
    pub fn index(self) -> usize {
        self.index as usize
    }
}
