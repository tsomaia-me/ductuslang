#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ExprRef(pub u32);

impl ExprRef {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}
