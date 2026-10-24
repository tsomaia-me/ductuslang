use crate::ids::FamilyId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AttrUnit {
    pub family: FamilyId,
    pub default_unit: usize,
}
