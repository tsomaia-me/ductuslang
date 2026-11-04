#[derive(Clone, Copy, Debug)]
pub(crate) struct Env {
    pub parent: u32,
    pub inst: u32,
    pub values: (u32, u32),
}
