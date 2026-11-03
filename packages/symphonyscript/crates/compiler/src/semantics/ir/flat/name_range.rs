use super::name_kind::NameKind;
use super::sym::Sym;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct NameRange {
    pub sym: Sym,
    pub lo: u32,
    pub hi: u32,
    pub kind: NameKind,
    pub faulty: bool,
}
