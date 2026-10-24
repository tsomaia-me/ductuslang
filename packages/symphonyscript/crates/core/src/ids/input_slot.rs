#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct InputSlot(pub u32);

impl InputSlot {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
