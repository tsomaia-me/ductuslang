#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DeclRef {
    None,
    Let(u32),
    Instance(u32),
    Duplicate,
}
