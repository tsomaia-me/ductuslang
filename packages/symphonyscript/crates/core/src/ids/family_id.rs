#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FamilyId {
    index: u32,
}

impl FamilyId {
    pub fn index(self) -> usize {
        self.index as usize
    }
}
