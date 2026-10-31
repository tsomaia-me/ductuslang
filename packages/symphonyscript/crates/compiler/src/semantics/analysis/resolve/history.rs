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
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::unwritten::Unwritten;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{CellWord, ExprId, HistoryForm, Ident, Loc, StateId};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn history(
        &mut self,
        expr: ExprId,
        (target, offset, fallback, form): (&Ident, ExprId, Option<ExprId>, HistoryForm),
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        if form == HistoryForm::Braced && !self.in_state_initializer(expr)? {
            return Ok(Some(Val::Error));
        }
        let offset_val = self.val(Query::Expr(offset, scope), need);
        let fallback_val = fallback.map(|fallback| self.val(Query::Expr(fallback, scope), need));
        let Some(cell) = self.history_target(target, scope, "history")? else {
            return Ok(Some(Val::Error));
        };
        let offset_val = some!(offset_val);
        let fallback_val = match fallback_val {
            Some(found) => Some(some!(found)),
            None => None,
        };
        if fallback.is_none()
            && form == HistoryForm::Braced
            && !matches!(cell, HistoryCell::Input(_))
        {
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
            None => match self.unwritten_fallback(cell, target, loc, scope, need)? {
                Some(Unwritten::Init(node)) => Some(node),
                Some(Unwritten::Cell) => None,
                Some(Unwritten::Fault) => return Ok(Some(Val::Error)),
                None => return Ok(None),
            },
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

    pub(in crate::semantics::analysis) fn stepped(
        &mut self,
        target: &Ident,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Option<Val>, Internal> {
        let Some(cell) = self.history_target(target, scope, "step")? else {
            return Ok(Some(Val::Error));
        };
        Ok(Some(self.scalar(Op::Stepped(cell), loc, scope)?))
    }

    fn history_target(
        &mut self,
        target: &Ident,
        scope: ScopeId,
        what: &str,
    ) -> Result<Option<HistoryCell>, Internal> {
        Ok(Some(match self.find(&target.text, scope)? {
            Found::Query(Query::Input(id)) => HistoryCell::Input(id),
            Found::Query(Query::State(id)) => HistoryCell::State(id),
            Found::Query(Query::Let(_, LetRef::Top(id))) => HistoryCell::Let(id),
            Found::Nothing | Found::Placeholder => return Ok(None),
            Found::Val(_) | Found::Query(_) | Found::Item(_) | Found::Decl(_) => {
                self.error(
                    Code::HistoryTarget,
                    target.loc,
                    scope,
                    format!(
                        "`{}` is not an input, a state or a reactive `let`, so it has no {what}",
                        target.text
                    ),
                )?;
                return Ok(None);
            }
        }))
    }

    fn unwritten_fallback(
        &mut self,
        cell: HistoryCell,
        target: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Unwritten>, Internal> {
        let query = match cell {
            HistoryCell::Input(_) => return Ok(Some(Unwritten::Cell)),
            HistoryCell::State(state) => match self.cell_init(state, need)? {
                Some(Some(found)) => return Ok(Some(found)),
                Some(None) => Query::State(state),
                None => return Ok(None),
            },
            HistoryCell::Let(id) => Query::Let(self.global_of_let(id), LetRef::Top(id)),
        };
        if self.in_progress(query) {
            let name = &target.text;
            let word = match cell {
                HistoryCell::State(state) => match self.hir.state(state)?.word {
                    CellWord::State => "state",
                    CellWord::Stream => "stream",
                    CellWord::Channel => "channel",
                },
                HistoryCell::Input(_) | HistoryCell::Let(_) => "state",
            };
            self.error_helped(
                Code::HistoryFallback,
                loc,
                scope,
                format!(
                    "this read of `{name}` lies on a cycle of history reads with its reader, and `{name}` has no init and the read no fallback, so it has no value at load"
                ),
                format!(
                    "give `{name}` an init, as in `{word} {name}(0) = …`, or this read a fallback, as in `{name}.past(1, 0)`"
                ),
            )?;
            return Ok(Some(Unwritten::Fault));
        }
        if self.bound(query, need).is_none() {
            return Ok(None);
        }
        Ok(Some(Unwritten::Cell))
    }

    pub(in crate::semantics::analysis) fn cell_init(
        &mut self,
        state: StateId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<Unwritten>>, Internal> {
        let Some(init) = self.hir.state(state)?.init else {
            return Ok(Some(None));
        };
        let global = self.source_global;
        let Some(val) = self.val(Query::Expr(init, global), need) else {
            return Ok(None);
        };
        let loc = self.hir.expr(init)?.loc;
        let Some(node) = self.boxed(&val, loc, global)? else {
            return Ok(Some(Some(Unwritten::Fault)));
        };
        let Some(sig) = self.sig(node, need) else {
            return Ok(None);
        };
        if sig.is_error() || sig.reactive || matches!(sig.kind, Kind::Str | Kind::Node) {
            return Ok(Some(Some(Unwritten::Fault)));
        }
        Ok(Some(Some(Unwritten::Init(node))))
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
        if let Some(init) = decl.init
            && !self.init_checked(init, need)?
        {
            return Ok(false);
        }
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
            if sig.kind == Kind::Str {
                self.error(Code::UnbuiltConstruct, loc, global, STRING_CELL)?;
            } else {
                self.error(
                    Code::StateValue,
                    loc,
                    global,
                    "a state never holds a node reference",
                )?;
            }
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

    pub(in crate::semantics::analysis) fn init_node(
        &mut self,
        binding: BindingId,
    ) -> Result<Option<NodeId>, Internal> {
        let BindingKind::State { state } = self.binding(binding)?.kind else {
            return Ok(None);
        };
        let Some(init) = self.hir.state(state)?.init else {
            return Ok(None);
        };
        let global = self.source_global;
        let Some(val) = self.memo.exprs.get(&(init, global)).cloned() else {
            return Ok(None);
        };
        let loc = self.hir.expr(init)?.loc;
        let Some(node) = self.boxed(&val, loc, global)? else {
            return Ok(None);
        };
        let sig = self.demand_sig(node)?;
        let usable =
            !sig.is_error() && !sig.reactive && !matches!(sig.kind, Kind::Str | Kind::Node);
        Ok(usable.then_some(node))
    }

    fn init_checked(&mut self, init: ExprId, need: &mut Vec<Query>) -> Result<bool, Internal> {
        let global = self.source_global;
        let Some(val) = self.val(Query::Expr(init, global), need) else {
            return Ok(false);
        };
        let Some(sigs) = self.element_sigs(&val, need) else {
            return Ok(false);
        };
        let loc = self.hir.expr(init)?.loc;
        if sigs.iter().any(Sig::is_error) {
            return Ok(true);
        }
        if sigs.iter().any(|sig| sig.reactive) {
            for node in val.nodes() {
                let reads = self.untypable_reads(node)?;
                self.typing.read_by_faults.extend(reads);
            }
            self.error(
                Code::ReactiveInitializer,
                loc,
                global,
                "a cell's init is a compile-time value and reads no cell",
            )?;
        } else if sigs.iter().any(|sig| sig.kind == Kind::Str) {
            self.error(Code::UnbuiltConstruct, loc, global, STRING_CELL)?;
        } else if sigs.iter().any(|sig| sig.kind == Kind::Node) {
            self.error(
                Code::StateValue,
                loc,
                global,
                "a cell never holds a node reference",
            )?;
        }
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

pub(in crate::semantics::analysis) const STRING_CELL: &str =
    "a cell holding a string (SPEC §3.3) is a construct this compiler does not build";
