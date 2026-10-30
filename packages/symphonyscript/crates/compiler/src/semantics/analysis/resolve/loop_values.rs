use super::bind_spec::BindSpec;
use crate::semantics::Internal;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{Ident, TypeRef};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn annotated_loop_value(
        &mut self,
        ty: &TypeRef,
        value: Val,
        name: &Ident,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        let Some(annotation) = self.annotation(ty) else {
            return Ok(Val::Error);
        };
        let mut sigs = Vec::new();
        for node in value.nodes() {
            sigs.push(self.demand_sig(node)?);
        }
        let spec = BindSpec {
            kind: BindingKind::Let,
            annotation: Some(&annotation),
            name: &name.text,
            loc: name.loc,
            scope,
        };
        let bound = self.bind(&value, &sigs, &spec)?;
        self.read(&bound, name.loc, scope)
    }
}
