use symphonyscript_core::{FamilyId, Scalar};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Suffix {
    None,
    Type(Scalar),
    Unit { family: FamilyId, unit: usize },
}
