#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct StrId(pub u32);

impl StrId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
