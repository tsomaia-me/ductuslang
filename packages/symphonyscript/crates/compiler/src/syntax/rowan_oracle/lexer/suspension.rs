#[derive(Clone)]
pub(super) struct Suspension {
    pub(super) offset: usize,
    pub(super) first_token: usize,
    pub(super) column: u32,
    pub(super) region: usize,
}
