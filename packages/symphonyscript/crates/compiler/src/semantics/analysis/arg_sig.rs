use symphonyscript_core::FamilyId;

use super::arg_reference::ArgReference;
use super::kind::Kind;
use super::width::Width;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ArgSig {
    pub kind: Kind,
    pub width: Width,
    pub family: Option<FamilyId>,
    pub node: ArgReference,
}
