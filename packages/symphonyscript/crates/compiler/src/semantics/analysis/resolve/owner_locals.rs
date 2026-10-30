use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::ir::{ExprKind, LetDecl, Param};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn owner_locals(
        &self,
        owner: LocalsOwner,
    ) -> Result<(&[Param], &[LetDecl]), Internal> {
        Ok(match owner {
            LocalsOwner::Fn(function) => {
                let decl = self.hir.fn_decl(function)?;
                (&decl.params, &decl.lets)
            }
            LocalsOwner::Pattern(pattern) => {
                let decl = self.hir.pattern(pattern)?;
                (&decl.params, &decl.lets)
            }
            LocalsOwner::For(expr) => {
                let ExprKind::For(for_expr) = &self.hir.expr(expr)?.kind else {
                    return Err(Internal::missed("for expression"));
                };
                let params = self
                    .memo
                    .loop_params
                    .get(&expr)
                    .ok_or(Internal::missed("loop names"))?;
                (params, &for_expr.lets)
            }
        })
    }
}
