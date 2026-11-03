#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TypeRec {
    pub name: u32,
    pub alias: u32,
    pub fields: (u32, u32),
    pub lo: u32,
    pub hi: u32,
}
