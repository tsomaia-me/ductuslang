use super::bindings::{ANNOTATED_ROWS, ANNOTATED_SHAPE, Rows};
use super::found::Found;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::answers::CellRead;
use crate::semantics::analysis::binding::Binding;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, Ident, Loc, StateId};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn history(
        &mut self,
        expr: ExprId,
        (target, offset, fallback): (&Ident, ExprId, Option<ExprId>),
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        if !self.in_state_initializer(expr)? {
            return Ok(Some(Val::Error));
        }
        let offset_val = self.val(Query::Expr(offset, scope), need);
        let fallback_val = fallback.map(|fallback| self.val(Query::Expr(fallback, scope), need));
        let cell = match self.find(&target.text, scope)? {
            Found::Query(Query::Input(id)) => HistoryCell::Input(id),
            Found::Query(Query::State(id)) => HistoryCell::State(id),
            Found::Query(Query::Let(_, LetRef::Top(id))) => HistoryCell::Let(id),
            Found::Nothing | Found::Placeholder => return Ok(Some(Val::Error)),
            Found::Val(_) | Found::Query(_) | Found::Item(_) | Found::Decl(_) => {
                self.error(
                    Code::HistoryTarget,
                    target.loc,
                    scope,
                    format!(
                        "`{}` is not an input, a state or a reactive `let`, so it has no history",
                        target.text
                    ),
                )?;
                return Ok(Some(Val::Error));
            }
        };
        let offset_val = some!(offset_val);
        let fallback_val = match fallback_val {
            Some(found) => Some(some!(found)),
            None => None,
        };
        if fallback.is_none() && !matches!(cell, HistoryCell::Input(_)) {
            self.error(
                Code::HistoryFallback,
                loc,
                scope,
                format!(
                    "a history read of a state or a `let` states its fallback, as in `{}{{1, 0}}`",
                    target.text
                ),
            )?;
            return Ok(Some(Val::Error));
        }
        let offset_loc = self.hir.expr(offset)?.loc;
        let offset_node = match offset_val {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(Code::HistoryOffset, offset_loc, scope, OFFSET)?;
                return Ok(Some(Val::Error));
            }
            Val::Error => return Ok(Some(Val::Error)),
        };
        let sig = some!(self.sig(offset_node, need));
        if sig.is_error() {
            return Ok(Some(Val::Error));
        }
        if sig.reactive || sig.kind != Kind::Int || sig.family.is_some() {
            self.error(Code::HistoryOffset, offset_loc, scope, OFFSET)?;
            return Ok(Some(Val::Error));
        }
        let offset = match some!(self.cval(Query::Eval(offset_node, EvalTy::Exact), need)) {
            CVal::Int(value) if value >= 1 => u64::try_from(value).unwrap_or(u64::MAX),
            CVal::Int(_) => {
                self.error(Code::HistoryOffset, offset_loc, scope, OFFSET)?;
                return Ok(Some(Val::Error));
            }
            CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_)
            | CVal::Array(_)
            | CVal::Error => return Ok(Some(Val::Error)),
        };
        if let HistoryCell::Input(input) = cell {
            some!(self.bound(Query::Input(input), need));
        }
        let fallback_loc = match fallback {
            Some(expr) => self.hir.expr(expr)?.loc,
            None => loc,
        };
        let fallback = match fallback_val {
            Some(Val::Error) => return Ok(Some(Val::Error)),
            Some(val) => match self.boxed(&val, fallback_loc, scope)? {
                Some(node) => Some(node),
                None => return Ok(Some(Val::Error)),
            },
            None => None,
        };
        Ok(Some(self.scalar(
            Op::History {
                cell,
                offset,
                fallback,
            },
            loc,
            scope,
        )?))
    }

    fn in_state_initializer(&mut self, expr: ExprId) -> Result<bool, Internal> {
        if self.memo.state_exprs.is_none() {
            let mut found = FastSet::default();
            for decl in &self.hir.states {
                let mut stack = vec![decl.value];
                while let Some(current) = stack.pop() {
                    crate::semantics::counted_walk::CountedWalk::HistoryInStateInitializer.step();
                    if found.insert(current) {
                        stack.extend(self.hir.expr(current)?.kind.children());
                    }
                }
            }
            self.memo.state_exprs = Some(found);
        }
        Ok(self
            .memo
            .state_exprs
            .as_ref()
            .is_some_and(|found| found.contains(&expr)))
    }

    pub(in crate::semantics::analysis) fn run_state(
        &mut self,
        state: StateId,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let decl = self.hir.state(state)?;
        let global = self.source_global;
        let Some(val) = self.val(Query::Expr(decl.value, global), need) else {
            return Ok(false);
        };
        let loc = self.hir.expr(decl.value)?.loc;
        let annotation = decl.ty.as_ref().and_then(|ty| self.annotation(ty));
        if decl.ty.is_some() && annotation.is_none() {
            self.memo.states.insert(state, BoundVal::Error);
            return Ok(true);
        }
        let Some(val) = self.as_bytes(val, annotation.as_ref(), loc, global, need)? else {
            return Ok(false);
        };
        let Some(init) = self.boxed(&val, loc, global)? else {
            self.memo.states.insert(state, BoundVal::Error);
            return Ok(true);
        };
        let Some(sig) = self.sig(init, need) else {
            return Ok(false);
        };
        if sig.is_error() {
            self.memo.states.insert(state, BoundVal::Error);
            return Ok(true);
        }
        if matches!(sig.kind, Kind::Str | Kind::Node) {
            let message = if sig.kind == Kind::Str {
                "a state never holds a string, which exists only at compile time"
            } else {
                "a state never holds a node reference"
            };
            self.error(Code::StateValue, loc, global, message)?;
            self.memo.states.insert(state, BoundVal::Error);
            return Ok(true);
        }
        let Some(found) = self.layout_of(init, need)? else {
            return Ok(false);
        };
        let layout = match found.shaped {
            Ok(layout) => layout,
            Err(fault) => {
                if let Some(fault) = fault {
                    self.error_at_node(fault.code, fault.node, fault.message)?;
                }
                self.memo.states.insert(state, BoundVal::Error);
                return Ok(true);
            }
        };
        if let Some(annotation) = annotation.as_ref()
            && annotation.capacity.is_some()
        {
            match self.annotated_rows(init, annotation, need)? {
                None => return Ok(false),
                Some(Rows::Agree) => {}
                Some(Rows::Faulty) => {
                    self.memo.states.insert(state, BoundVal::Error);
                    return Ok(true);
                }
                Some(Rows::Differ) => {
                    self.error(Code::TypeMismatch, loc, global, ANNOTATED_ROWS)?;
                    self.memo.states.insert(state, BoundVal::Error);
                    return Ok(true);
                }
            }
        }
        if let Some(annotation) = annotation.as_ref()
            && annotation.capacity.is_none()
            && let Some((length, rows)) = annotation.lengths.split_last()
        {
            let differs = match &layout {
                Some(held) => {
                    held.inner() != rows
                        || (found.certain && !held.variable && held.outer() != Some(*length))
                }
                None => true,
            };
            if differs {
                self.error(Code::TypeMismatch, loc, global, ANNOTATED_SHAPE)?;
                self.memo.states.insert(state, BoundVal::Error);
                return Ok(true);
            }
        }
        let layout = match annotation.as_ref() {
            Some(annotation) if annotation.capacity.is_some() => {
                Some(Layout::variable(annotation.lengths.clone()))
            }
            Some(annotation) if !annotation.lengths.is_empty() => {
                Some(Layout::fixed(annotation.lengths.clone()))
            }
            Some(_) | None => layout,
        };
        let name = decl
            .name
            .as_ref()
            .map(|name| name.text.clone())
            .unwrap_or_default();
        let binding = self.push_binding(Binding {
            kind: BindingKind::State { state },
            init,
            annotation: annotation.as_ref().map(|annotation| annotation.scalar),
            name,
            loc: decl.loc,
        })?;
        if let Some(layout) = layout {
            self.memo.binding_layouts.insert(binding, layout);
        }
        if annotation.as_ref().is_some_and(|annotation| {
            annotation.capacity.is_some() || !annotation.lengths.is_empty()
        }) {
            self.memo.settled.insert(binding);
        }
        if let Some(capacity) = annotation
            .as_ref()
            .and_then(|annotation| annotation.capacity)
        {
            self.memo.binding_capacities.insert(binding, capacity);
        }
        self.memo
            .states
            .insert(state, BoundVal::Scalar(Bound::Cell(binding)));
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn history_reads(
        &self,
        init: NodeId,
    ) -> Result<Vec<(NodeId, HistoryCell)>, Internal> {
        let mut found = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![init];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::HistoryHistoryReads.step();
            if !seen.insert(node) {
                continue;
            }
            match self.cell_read(node)? {
                CellRead::History { cell, fallback } => {
                    found.push((node, cell));
                    stack.extend(fallback);
                }
                CellRead::Binding(binding) => {
                    if self.inlined_binding(binding)? {
                        stack.push(self.binding(binding)?.init);
                    }
                }
                CellRead::Named(inner) => stack.push(inner),
                CellRead::Operands(operands) => stack.extend(operands),
            }
        }
        found.sort_by_key(|(node, _)| *node);
        Ok(found)
    }

    fn inlined_binding(&self, binding: BindingId) -> Result<bool, Internal> {
        let record = self.binding(binding)?;
        Ok(match record.kind {
            BindingKind::Param => true,
            BindingKind::Let => self.node(record.init)?.scope != self.source_global,
            BindingKind::Input { .. } | BindingKind::State { .. } | BindingKind::Attr { .. } => {
                false
            }
        })
    }
}

const OFFSET: &str = "the offset of a history read is a positive integer known at compile time";
