#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DeclRef {
    None,
    Let(u32),
    Cell(u32),
    Instance(u32),
    Duplicate,
    Param(u32),
    Fn(u32),
}
