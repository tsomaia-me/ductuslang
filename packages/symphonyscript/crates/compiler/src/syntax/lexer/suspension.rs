#[derive(Clone)]
pub(crate) struct Suspension {
    pub(crate) offset: usize,
    pub(crate) first: usize,
    pub(crate) column: u32,
    pub(crate) stack: Vec<u32>,
}
