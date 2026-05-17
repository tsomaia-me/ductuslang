#[derive(Debug)]
pub enum FreeListError {
    DoubleFree,
    InvalidSlot,
}
