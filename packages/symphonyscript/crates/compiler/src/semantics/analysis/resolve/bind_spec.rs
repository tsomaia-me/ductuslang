use super::annotation::Annotation;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::ir::Loc;

pub(crate) struct BindSpec<'n> {
    pub kind: BindingKind,
    pub annotation: Option<&'n Annotation>,
    pub name: &'n str,
    pub loc: Loc,
    pub scope: ScopeId,
}
