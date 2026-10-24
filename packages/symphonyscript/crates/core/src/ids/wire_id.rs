#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WireId(pub u32);

impl WireId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
