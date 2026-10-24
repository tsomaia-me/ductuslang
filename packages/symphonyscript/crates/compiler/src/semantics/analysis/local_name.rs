#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LocalName {
    Param(u32),
    Let(u32),
}
