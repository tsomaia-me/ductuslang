use symphonyscript_core::Value;

use super::iteration::Iteration;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::evaluation::own_type;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Iterable, Loc, LoopVar, Number, Suffix};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn loop_iteration(
        &mut self,
        names: &[LoopVar],
        iterable: Iterable,
        scope: ScopeId,
    ) -> Result<Option<Iteration>, Internal> {
        if let (Iterable::Range(start, end), [_, second, ..]) = (iterable, names) {
            let range = self.hir.expr(start)?.loc.to(self.hir.expr(end)?.loc);
            let loc = second.name.as_ref().map_or(range, |name| name.loc);
            self.error_helped(
                Code::IterableType,
                loc,
                scope,
                RANGE_NAMES.to_string(),
                "write one name over a range, as in `for i in 0..4`".to_string(),
            )?;
            return Ok(None);
        }
        self.iteration(iterable, scope)
    }

    pub(in crate::semantics::analysis) fn loop_scopes(
        &mut self,
        names: &[LoopVar],
        (value, position): (Val, usize),
        scope: ScopeId,
    ) -> Result<ScopeId, Internal> {
        let bound: Vec<(&LoopVar, Val)> = match names {
            [] => Vec::new(),
            [one] => vec![(one, value)],
            [index, element, ..] => {
                let position =
                    i128::try_from(position).map_err(|_| Internal::missed("a loop position"))?;
                let index_value = match &index.name {
                    Some(name) => Val::Scalar(self.push_node(
                        Op::Number(Number::Int(position), Suffix::None),
                        name.loc,
                        scope,
                    )?),
                    None => Val::Error,
                };
                vec![(index, index_value), (element, value)]
            }
        };
        let mut inner = scope;
        for (var, value) in bound {
            let Some(name) = &var.name else {
                continue;
            };
            let value = match &var.ty {
                Some(ty) => self.annotated_loop_value(ty, value, name, inner)?,
                None => value,
            };
            let chain = self.chain(inner)?;
            inner = self.scope(
                ScopeKind::ForVar {
                    name: name.text.clone(),
                    value,
                },
                Some(inner),
                chain,
            )?;
        }
        Ok(inner)
    }

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
                if let Val::Scalar(node) = val
                    && self.arrays.contains(&node)
                {
                    return self.computed_iteration(node, loc, scope);
                }
                let array = match &val {
                    Val::Array(array) if array.shape.depth() == 1 => array.clone(),
                    Val::Error => return Ok(None),
                    Val::Scalar(node) if self.demand_sig(*node)?.is_error() => return Ok(None),
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
                        self.error(Code::ReactiveIterable, loc, scope, REACTIVE)?;
                        return Ok(None);
                    }
                }
                let mut bare = false;
                for node in &array.elements {
                    if self.demand_sig(*node)?.kind == Kind::Bare
                        && !matches!(self.node(*node)?.op, Op::Use(_))
                    {
                        self.unplaced_placeholder(*node)?;
                        bare = true;
                    }
                }
                if bare {
                    return Ok(None);
                }
                for node in &array.elements {
                    let sig = self.demand_sig(*node)?;
                    if sig.family.is_none() && !sig.is_error() {
                        self.demand_cval(*node, own_type(&sig, EvalTy::Default))?;
                    }
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

    fn computed_iteration(
        &mut self,
        node: NodeId,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Option<Iteration>, Internal> {
        if self.demand_sig(node)?.reactive {
            let reads = self.untypable_reads(node)?;
            self.typing.read_by_faults.extend(reads);
            self.error(Code::ReactiveIterable, loc, scope, REACTIVE)?;
            return Ok(None);
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
            | CVal::Node(_) => {
                self.error(Code::IterableType, loc, scope, ITERABLE)?;
                return Ok(None);
            }
        };
        let lengths = array.lengths();
        if lengths.len() != 1 {
            self.error(Code::IterableType, loc, scope, ITERABLE)?;
            return Ok(None);
        }
        self.require_shape(node, lengths, Code::IterableType, ITERABLE);
        Ok(Some(Iteration::Elements {
            base: node,
            length: array.len(),
            next: 0,
            loc,
        }))
    }

    pub(in crate::semantics::analysis) fn computed_element(
        &mut self,
        base: NodeId,
        position: usize,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        let position =
            i64::try_from(position).map_err(|_| Internal::missed("an element's position"))?;
        let index = self.push_node(Op::Const(Value::I64(position)), loc, scope)?;
        self.memo.projected_indexes.insert(index);
        let element = self.push_node(Op::Element { base, index }, loc, scope)?;
        self.memo.iterated_elements += 1;
        let sig = self.demand_sig(element)?;
        if sig.family.is_none() && !sig.is_error() {
            self.demand_cval(element, own_type(&sig, EvalTy::Default))?;
        }
        Ok(Val::Scalar(self.chosen_read(element, base, loc, scope)?))
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
            | ExprKind::IndexFromEnd(..)
            | ExprKind::Slice { .. }
            | ExprKind::History { .. }
            | ExprKind::Call(..)
            | ExprKind::Unary(..)
            | ExprKind::Binary(..)
            | ExprKind::If(..)
            | ExprKind::Match { .. }
            | ExprKind::For(_) => some!(self.computed_bound(expr, scope)?),
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
        if sig.kind == Kind::Any {
            return Ok(match self.demand_cval(node, EvalTy::Exact)? {
                CVal::Int(value) => Some(value),
                CVal::Error => None,
                CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_) => {
                    self.error(Code::RangeBound, loc, scope, "a range bound is an integer")?;
                    None
                }
            });
        }
        if sig.kind != Kind::Int {
            let unless_values = self.untypable_values(node)?;
            self.error_unless_values(
                Code::RangeBound,
                loc,
                scope,
                "a range bound is an integer",
                None,
                unless_values,
            )?;
            self.demand_cval(node, own_type(&sig, EvalTy::Default))?;
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

const REACTIVE: &str = "a `for` iterates over a compile-time value, and this one reads a cell";

const RANGE_NAMES: &str =
    "a range binds one name; two names iterate an array, its positions and its elements";
