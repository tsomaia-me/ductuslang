use super::iteration::Iteration;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::shape::Shape;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Iterable, Loc, Number, Suffix};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn iteration(
        &mut self,
        iterable: Iterable,
        scope: ScopeId,
    ) -> Result<Option<Iteration>, Internal> {
        Ok(match iterable {
            Iterable::Range(start, end) => {
                let first = some!(self.range_bound(start, scope)?);
                let last = some!(self.range_bound(end, scope)?);
                let loc = self.hir.expr(start)?.loc.to(self.hir.expr(end)?.loc);
                Some(Iteration::Range {
                    next: first,
                    end: last,
                    loc,
                })
            }
            Iterable::Values(expr) => {
                let val = self.demand_val(Query::Expr(expr, scope))?;
                let loc = self.hir.expr(expr)?.loc;
                let val = some!(self.iterated(val, loc, scope)?);
                let array = match &val {
                    Val::Array(array) if array.shape.depth() == 1 => array.clone(),
                    Val::Error => return Ok(None),
                    Val::Array(_) | Val::Scalar(_) => {
                        self.error(Code::IterableType, loc, scope, ITERABLE)?;
                        return Ok(None);
                    }
                };
                for node in &array.elements {
                    let sig = self.demand_sig(*node)?;
                    if sig.reactive {
                        for element in &array.elements {
                            let reads = self.untypable_reads(*element)?;
                            self.typing.read_by_faults.extend(reads);
                        }
                        self.error(
                            Code::ReactiveIterable,
                            loc,
                            scope,
                            "a `for` iterates over a compile-time value, and this one reads a cell",
                        )?;
                        return Ok(None);
                    }
                }
                let mut bare = false;
                for node in &array.elements {
                    if self.demand_sig(*node)?.kind == Kind::Bare
                        && !matches!(self.node(*node)?.op, Op::Use(_))
                    {
                        self.unplaced_variant(*node)?;
                        bare = true;
                    }
                }
                if bare {
                    return Ok(None);
                }
                let group = self.value_group(expr, &val, scope)?;
                let mut values = Vec::with_capacity(array.elements.len());
                for element in &array.elements {
                    values.push(Val::Scalar(self.chosen_read(*element, group, loc, scope)?));
                }
                Some(Iteration::Values {
                    values,
                    next: 0,
                    loc,
                })
            }
        })
    }

    fn iterated(&mut self, val: Val, loc: Loc, scope: ScopeId) -> Result<Option<Val>, Internal> {
        let Val::Scalar(node) = val else {
            return Ok(Some(val));
        };
        if !self.arrays.contains(&node) {
            return Ok(Some(val));
        }
        let array = match self.demand_cval(node, EvalTy::Default)? {
            CVal::Array(array) => array,
            CVal::Error => return Ok(None),
            CVal::Int(_)
            | CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_) => return Ok(Some(val)),
        };
        let shape = Shape::from_lengths(&array.lengths());
        let nodes = self.projected(node, &shape, (Code::IterableType, ITERABLE), loc, scope)?;
        Ok(Some(Val::array(shape, nodes)))
    }

    fn range_bound(&mut self, expr: ExprId, scope: ScopeId) -> Result<Option<i128>, Internal> {
        let value = match self.hir.expr(expr)?.kind {
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
            | ExprKind::Match { .. } => some!(self.computed_bound(expr, scope)?),
        };
        if i64::try_from(value).is_err() {
            let loc = self.hir.expr(expr)?.loc;
            self.error(
                Code::RangeBound,
                loc,
                scope,
                format!("the range bound {value} does not fit `i64`"),
            )?;
            return Ok(None);
        }
        Ok(Some(value))
    }

    fn computed_bound(&mut self, expr: ExprId, scope: ScopeId) -> Result<Option<i128>, Internal> {
        let loc = self.hir.expr(expr)?.loc;
        let node: NodeId = match self.demand_val(Query::Expr(expr, scope))? {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(
                    Code::RangeBound,
                    loc,
                    scope,
                    "a range bound is an integer, not an array",
                )?;
                return Ok(None);
            }
            Val::Error => return Ok(None),
        };
        let sig = self.demand_sig(node)?;
        if sig.is_error() {
            return Ok(None);
        }
        if sig.reactive {
            self.error(
                Code::ReactiveIterable,
                loc,
                scope,
                "a `for` iterates over a compile-time value, and this bound reads a cell",
            )?;
            return Ok(None);
        }
        if sig.family.is_some() {
            self.error(
                Code::UnitOperation,
                loc,
                scope,
                "a unit value is not a range bound",
            )?;
            return Ok(None);
        }
        if sig.kind != Kind::Int {
            self.error(Code::RangeBound, loc, scope, "a range bound is an integer")?;
            return Ok(None);
        }
        Ok(match self.demand_cval(node, EvalTy::Exact)? {
            CVal::Int(value) => Some(value),
            CVal::Array(_) => {
                self.error(
                    Code::RangeBound,
                    loc,
                    scope,
                    "a range bound is an integer, not an array",
                )?;
                None
            }
            CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_)
            | CVal::Error => None,
        })
    }
}

const ITERABLE: &str = "a `for` iterates over a range or an array of single values";
