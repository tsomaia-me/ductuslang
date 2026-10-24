#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EnumId {
    index: u32,
}

impl EnumId {
    pub fn index(self) -> usize {
        self.index as usize
    }
}
