use super::binder_kind::BinderKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Binder {
    pub name: u32,
    pub lo: u32,
    pub hi: u32,
    pub kind: BinderKind,
    pub annotation: u32,
    pub owner: u32,
    pub default: u32,
    pub is_default: bool,
}
