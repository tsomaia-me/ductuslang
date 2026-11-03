#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct FieldRec {
    pub name: u32,
    pub ty: u32,
    pub lo: u32,
    pub hi: u32,
}
