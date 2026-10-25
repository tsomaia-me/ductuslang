#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NativeId {
    index: u32,
}

impl NativeId {
    pub(crate) fn new(index: u32) -> Self {
        Self { index }
    }

    pub fn index(self) -> usize {
        self.index as usize
    }
}
