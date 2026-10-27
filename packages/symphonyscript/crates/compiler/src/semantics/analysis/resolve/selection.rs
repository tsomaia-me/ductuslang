use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
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
                let mut labels = Vec::with_capacity(arms.len());
                for arm in arms {
                    labels.push(match &self.hir.expr(arm.label)?.kind {
                        ExprKind::Name(name) => self.bare_label(arm.label, name, scope)?,
                        _ => match some!(self.val(Query::Expr(arm.label, scope), need)) {
                            Val::Scalar(label) => label,
                            Val::Array(_) | Val::Error => return Ok(Some(None)),
                        },
                    });
                }
                let Some(ty) = self.choice_ty(node, &sig, &labels)? else {
                    return Ok(Some(None));
                };
                let value = some!(self.cval(Query::Eval(node, ty), need));
                if value.is_error() {
                    return Ok(Some(None));
                }
                for (arm, label) in arms.iter().zip(labels) {
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

    pub(super) fn pass_over(
        &mut self,
        kind: &ExprKind,
        chosen: ExprId,
        scope: ScopeId,
    ) -> Result<(), Internal> {
        let branches: Vec<ExprId> = match kind {
            ExprKind::If(_, then, otherwise) => vec![*then, *otherwise],
            ExprKind::Match {
                arms, otherwise, ..
            } => arms.iter().map(|arm| arm.value).chain(*otherwise).collect(),
            _ => Vec::new(),
        };
        let mut stack: Vec<ExprId> = branches
            .into_iter()
            .filter(|branch| *branch != chosen)
            .collect();
        while let Some(expr) = stack.pop() {
            let kind = &self.hir.expr(expr)?.kind;
            if let ExprKind::Call(..) = kind {
                self.memo.passed_over_calls.insert((expr, scope));
            }
            stack.extend(kind.children());
        }
        Ok(())
    }

    pub(super) fn bare_label(
        &mut self,
        label: ExprId,
        name: &str,
        scope: ScopeId,
    ) -> Result<NodeId, Internal> {
        if let Some(node) = self.memo.bare_labels.get(&(label, scope)) {
            return Ok(*node);
        }
        let loc = self.hir.expr(label)?.loc;
        let node = self.push_node(Op::Bare(String::from(name)), loc, scope)?;
        self.memo.bare_labels.insert((label, scope), node);
        Ok(node)
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
