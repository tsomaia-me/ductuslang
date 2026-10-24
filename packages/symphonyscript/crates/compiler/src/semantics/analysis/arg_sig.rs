use symphonyscript_core::FamilyId;

use super::kind::Kind;
use super::width::Width;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ArgSig {
    pub kind: Kind,
    pub width: Width,
    pub family: Option<FamilyId>,
}
