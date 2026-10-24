#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct CellId(pub u32);

impl CellId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
