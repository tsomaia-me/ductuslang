use symphonyscript_core::FamilyId;

use super::ids::ScopeId;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RecFamily {
    Probing(ScopeId),
    Settling {
        family: Option<FamilyId>,
        scope: ScopeId,
    },
    Known(Option<FamilyId>),
    Failed,
}
