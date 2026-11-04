#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Need {
    Body(u32),
    Local(u32, u32, u32),
}
