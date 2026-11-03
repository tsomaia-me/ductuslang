#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Part {
    Text(u32, u32),
    Hole(u32),
}
