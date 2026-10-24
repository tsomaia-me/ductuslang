#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct InstanceId(pub u32);

impl InstanceId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
