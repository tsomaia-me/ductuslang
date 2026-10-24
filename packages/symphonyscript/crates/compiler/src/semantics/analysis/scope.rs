use super::chain::Chain;
use super::ids::ScopeId;
use super::scope_kind::ScopeKind;

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Scope {
    pub kind: ScopeKind,
    pub parent: Option<ScopeId>,
    pub chain: Chain,
    pub template: Option<ScopeId>,
}
