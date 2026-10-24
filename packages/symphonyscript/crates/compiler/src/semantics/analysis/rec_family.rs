use symphonyscript_core::FamilyId;

use super::ids::ScopeId;
use super::kind::Kind;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RecFamily {
    Probing(ScopeId),
    Settling {
        kind: Kind,
        family: Option<FamilyId>,
        scope: ScopeId,
    },
    Known(Kind, Option<FamilyId>),
    Failed,
}
