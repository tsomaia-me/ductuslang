#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AttrId(pub u32);

impl AttrId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
