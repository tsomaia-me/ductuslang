use super::iteration::Iteration;
use crate::diagnostics::Code;
use crate::semantics::analysis::cval::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::Kind;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Iterable, Number, Suffix};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn iteration(
        &mut self,
        iterable: Iterable,
        scope: ScopeId,
    ) -> Option<Iteration> {
        match iterable {
            Iterable::Range(start, end) => {
                let first = self.range_bound(start, scope)?;
                let last = self.range_bound(end, scope)?;
                let loc = self.hir.expr(start).loc.to(self.hir.expr(end).loc);
                Some(Iteration::Range {
                    next: first,
                    end: last,
                    loc,
                })
            }
            Iterable::Values(expr) => {
                let val = self.demand_val(Query::Expr(expr, scope));
                let loc = self.hir.expr(expr).loc;
                let array = match &val {
                    Val::Array(array) if array.shape.depth() == 1 => array.clone(),
                    Val::Error => return None,
                    Val::Array(_) | Val::Scalar(_) => {
                        self.error(
                            Code::IterableType,
                            loc,
                            scope,
                            "a `for` iterates over a range or an array of single values",
                        );
                        return None;
                    }
                };
                let mut bare = false;
                for node in &array.elements {
                    if self.demand_sig(*node).kind == Kind::Bare {
                        self.unplaced_variant(*node);
                        bare = true;
                    }
                }
                if bare {
                    return None;
                }
                for node in &array.elements {
                    let sig = self.demand_sig(*node);
                    if sig.reactive {
                        self.error(
                            Code::ReactiveIterable,
                            loc,
                            scope,
                            "a `for` iterates over a compile-time value, and this one reads a cell",
                        );
                        return None;
                    }
                }
                let values = array
                    .elements
                    .iter()
                    .map(|node| Val::Scalar(*node))
                    .collect();
                Some(Iteration::Values {
                    values,
                    next: 0,
                    loc,
                })
            }
        }
    }

    fn range_bound(&mut self, expr: ExprId, scope: ScopeId) -> Option<i128> {
        let value = match self.hir.expr(expr).kind {
            ExprKind::Number(Number::Int(value), Suffix::None) => value,
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Name(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Convert(..)
            | ExprKind::Array(_)
            | ExprKind::Field(..)
            | ExprKind::Index(..)
            | ExprKind::Call(..)
            | ExprKind::Unary(..)
            | ExprKind::Binary(..)
            | ExprKind::If(..)
            | ExprKind::Match { .. } => self.computed_bound(expr, scope)?,
        };
        if i64::try_from(value).is_err() {
            let loc = self.hir.expr(expr).loc;
            self.error(
                Code::RangeBound,
                loc,
                scope,
                format!("the range bound {value} does not fit `i64`"),
            );
            return None;
        }
        Some(value)
    }

    fn computed_bound(&mut self, expr: ExprId, scope: ScopeId) -> Option<i128> {
        let loc = self.hir.expr(expr).loc;
        let node: NodeId = match self.demand_val(Query::Expr(expr, scope)) {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(
                    Code::RangeBound,
                    loc,
                    scope,
                    "a range bound is an integer, not an array",
                );
                return None;
            }
            Val::Error => return None,
        };
        let sig = self.demand_sig(node);
        if sig.is_error() {
            return None;
        }
        if sig.reactive {
            self.error(
                Code::ReactiveIterable,
                loc,
                scope,
                "a `for` iterates over a compile-time value, and this bound reads a cell",
            );
            return None;
        }
        if sig.family.is_some() {
            self.error(
                Code::UnitOperation,
                loc,
                scope,
                "a unit value is not a range bound",
            );
            return None;
        }
        if sig.kind != Kind::Int {
            self.error(Code::RangeBound, loc, scope, "a range bound is an integer");
            return None;
        }
        match self.demand_cval(node, EvalTy::Exact) {
            CVal::Int(value) => Some(value),
            CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_)
            | CVal::Error => None,
        }
    }
}
