use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind};

impl Engine<'_> {
    pub(super) fn selected(
        &mut self,
        kind: &ExprKind,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<ExprId>>, Internal> {
        match kind {
            ExprKind::If(condition, then, otherwise) => {
                let Some(value) =
                    some!(self.known_choice(*condition, Some(EvalTy::Bool), scope, need)?)
                else {
                    return Ok(Some(None));
                };
                Ok(Some(match value {
                    CVal::Bool(true) => Some(*then),
                    CVal::Bool(false) => Some(*otherwise),
                    _ => None,
                }))
            }
            ExprKind::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let Some(Val::Scalar(node)) = self.val(Query::Expr(*scrutinee, scope), need) else {
                    return Ok(
                        if self.val(Query::Expr(*scrutinee, scope), need).is_none() {
                            None
                        } else {
                            Some(None)
                        },
                    );
                };
                let sig = some!(self.sig(node, need));
                if sig.reactive || sig.is_error() || sig.frame {
                    return Ok(Some(None));
                }
                let Some(ty) = self.scrutinee_ty(node, &sig) else {
                    return Ok(Some(None));
                };
                let value = some!(self.cval(Query::Eval(node, ty), need));
                if value.is_error() {
                    return Ok(Some(None));
                }
                for arm in arms {
                    let label = match &self.hir.expr(arm.label)?.kind {
                        ExprKind::Name(name) => {
                            let loc = self.hir.expr(arm.label)?.loc;
                            self.push_node(Op::Bare(name.clone()), loc, scope)?
                        }
                        _ => match some!(self.val(Query::Expr(arm.label, scope), need)) {
                            Val::Scalar(label) => label,
                            Val::Array(_) | Val::Error => return Ok(Some(None)),
                        },
                    };
                    let label_value = some!(self.cval(Query::Eval(label, ty), need));
                    if label_value.is_error() {
                        return Ok(Some(None));
                    }
                    if label_value == value {
                        return Ok(Some(Some(arm.value)));
                    }
                }
                Ok(Some(*otherwise))
            }
            _ => Ok(Some(None)),
        }
    }

    fn known_choice(
        &mut self,
        selector: ExprId,
        ty: Option<EvalTy>,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<CVal>>, Internal> {
        let val = some!(self.val(Query::Expr(selector, scope), need));
        let Val::Scalar(node) = val else {
            return Ok(Some(None));
        };
        let sig = some!(self.sig(node, need));
        if sig.reactive || sig.is_error() || sig.frame || sig.kind != Kind::Bool {
            return Ok(Some(None));
        }
        let ty = ty.unwrap_or(EvalTy::Bool);
        let value = some!(self.cval(Query::Eval(node, ty), need));
        Ok(Some(Some(value)))
    }
}
