use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use symphonyscript_core::{
    EnumDecl, EnumId, FamilyDecl, FamilyId, Limits, NodeKind, NodeKindDecl, UnitDecl, Vocabulary,
    WireKind, WireKindDecl,
};

use super::answers::Dimensions;
use super::binding::Binding;
use super::bound::Bound;
use super::bound_val::BoundVal;
use super::c_val::CVal;
use super::chain::Chain;
use super::destination::Destination;
use super::fast_hash::{FastMap, FastSet};
use super::ids::{BindingId, InstanceId, NodeId, ScopeId};
use super::index::index;
use super::instance::Instance;
use super::item_ref::ItemRef;
use super::let_ref::LetRef;
use super::memo::Memo;
use super::node::Node;
use super::op::Op;
use super::path_parent::PathParent;
use super::pattern_use::PatternUse;
use super::query::Query;
use super::scope::Scope;
use super::scope_kind::ScopeKind;
use super::sig::Sig;
use super::site::Site;
use super::val::Val;
use super::wire::Wire;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::collection::{Collected, Variants};
use crate::semantics::ir::{ExprId, FnId, Hir, Loc, PlacementId};
use crate::semantics::reporting::{Report, grouped};

const ID_SPACE: u64 = 1 << 32;

const NAME_CODES: [Code; 11] = [
    Code::UnknownName,
    Code::NotAValue,
    Code::NotCallable,
    Code::UnknownAttribute,
    Code::UnknownParameter,
    Code::UnknownVariant,
    Code::UnknownType,
    Code::HistoryTarget,
    Code::DestinationType,
    Code::UnknownFlag,
    Code::MisplacedTo,
];

const VALUE_CODES: [Code; 6] = [
    Code::IndexOutOfRange,
    Code::DoesNotFit,
    Code::IntegerOverflow,
    Code::ConstantOverflow,
    Code::LengthLimit,
    Code::RaggedArray,
];

pub(crate) struct Engine<'a> {
    pub hir: &'a Hir,
    pub vocabulary: &'a Vocabulary,
    pub collected: &'a Collected,
    pub variants: &'a Variants,
    pub limits: Limits,
    pub id_space: u64,
    pub ids_exhausted: bool,
    pub stop: Option<super::stop::Stop>,
    pub inferred: bool,
    pub report: Report,
    pub scopes: Vec<Scope>,
    pub nodes: Vec<Node>,
    pub slots: Vec<super::slot::Slot>,
    pub arrays: FastSet<NodeId>,
    pub bindings: Vec<Binding>,
    pub instances: Vec<Instance>,
    pub wires: Vec<Wire>,
    pub uses: Vec<PatternUse>,
    pub top_instances: BTreeMap<PlacementId, InstanceId>,
    pub top_level: Vec<InstanceId>,
    pub source_global: ScopeId,
    pub prelude_global: ScopeId,
    pub names_global: ScopeId,
    pub prelude_names_global: ScopeId,
    pub memo: Memo,
    pub prebuilt: FastMap<(Site, ScopeId), ItemRef>,
    pub counters: FastMap<(PathParent, &'a str), u32>,
    pub body_queue: VecDeque<InstanceId>,
    pub expanded: u64,
    pub steps: u64,
    pub limit_reached: bool,
    pub broken_bundles: FastSet<usize>,
    pub condition_links: Vec<super::condition_link::ConditionLink>,
    pub gates: Vec<(crate::semantics::ir::ExprId, ScopeId)>,
    pub gate_set: FastSet<(crate::semantics::ir::ExprId, ScopeId)>,
    pub unknown_children: FastSet<InstanceId>,
    pub unknown_top: bool,
    pub unknown_placement: bool,
    pub unknown_wires: bool,
    pub unknown_destinations: FastSet<symphonyscript_core::WireKind>,
    pub active: Option<super::evaluation::Machine>,
    pub machines: FastMap<super::evaluation::MachineKey, super::evaluation::Machine>,
    pub frame_need: Vec<(NodeId, super::eval_ty::EvalTy)>,
    pub frame_pushed: bool,
    pub fault_frame: Option<usize>,
    pub sink: super::sink::Sink,
    pub fault_count: u32,
    pub call_count: u64,
    pub inlined: u64,
    pub evaluated_bodies: FastSet<FnId>,
    pub evaluated_defaults: FastSet<(FnId, usize)>,
    pub inline_limit_reported: bool,
    pub inline_warning_reported: bool,
    pub cycle_spans: Vec<Loc>,
    pub call_limit_reported: bool,
    pub call_warning_reported: bool,
    pub typing: super::infer::Typing,
    pub deferred: Vec<super::deferred::Deferred>,
    #[cfg(test)]
    pub run_query: fn(&mut Engine<'a>, Query, &mut Vec<Query>) -> Result<bool, Internal>,
    waiting: FastSet<Query>,
    cyclic: FastSet<(Query, Query)>,
    cycle_members: FastSet<Query>,
    pub(super) candidate_edges: FastMap<(Query, Query), (ExprId, ItemRef, String)>,
    pub(super) open_candidates: FastMap<(ExprId, ItemRef, String), Rc<[Query]>>,
    drawn_candidate_cycles: FastSet<(ItemRef, String)>,
    pub(super) found_cycles: Vec<Vec<(Loc, String)>>,
    pub(super) declaration_reads: std::cell::RefCell<FastSet<(Query, Query)>>,
    reader: Option<Query>,
    pub(super) current: Option<Query>,
}

impl<'a> Engine<'a> {
    pub fn new(
        hir: &'a Hir,
        vocabulary: &'a Vocabulary,
        collected: &'a Collected,
        variants: &'a Variants,
        limits: Limits,
    ) -> Self {
        let mut engine = Self {
            hir,
            vocabulary,
            collected,
            variants,
            limits,
            id_space: ID_SPACE,
            ids_exhausted: false,
            stop: None,
            inferred: false,
            report: Report::default(),
            scopes: Vec::new(),
            nodes: Vec::new(),
            slots: Vec::new(),
            arrays: FastSet::default(),
            bindings: Vec::new(),
            instances: Vec::new(),
            wires: Vec::new(),
            uses: Vec::new(),
            top_instances: BTreeMap::new(),
            top_level: Vec::new(),
            source_global: ScopeId(0),
            prelude_global: ScopeId(1),
            names_global: ScopeId(2),
            prelude_names_global: ScopeId(3),
            memo: Memo::default(),
            prebuilt: FastMap::default(),
            counters: FastMap::default(),
            body_queue: VecDeque::new(),
            expanded: 0,
            steps: 0,
            limit_reached: false,
            broken_bundles: FastSet::default(),
            condition_links: Vec::new(),
            gates: Vec::new(),
            gate_set: FastSet::default(),
            unknown_children: FastSet::default(),
            unknown_top: false,
            unknown_placement: false,
            unknown_wires: false,
            unknown_destinations: FastSet::default(),
            active: None,
            machines: FastMap::default(),
            frame_need: Vec::new(),
            frame_pushed: false,
            fault_frame: None,
            sink: super::sink::Sink::Reached,
            fault_count: 0,
            call_count: 0,
            inlined: 0,
            evaluated_bodies: FastSet::default(),
            evaluated_defaults: FastSet::default(),
            inline_limit_reported: false,
            inline_warning_reported: false,
            cycle_spans: Vec::new(),
            call_limit_reported: false,
            call_warning_reported: false,
            typing: super::infer::Typing::default(),
            deferred: Vec::new(),
            #[cfg(test)]
            run_query: Engine::run,
            waiting: FastSet::default(),
            cyclic: FastSet::default(),
            cycle_members: FastSet::default(),
            candidate_edges: FastMap::default(),
            open_candidates: FastMap::default(),
            drawn_candidate_cycles: FastSet::default(),
            found_cycles: Vec::new(),
            declaration_reads: std::cell::RefCell::default(),
            reader: None,
            current: None,
        };
        engine.source_global = engine.push_scope(
            ScopeKind::Global { prelude: false },
            None,
            Chain::default(),
            None,
        );
        engine.prelude_global = engine.push_scope(
            ScopeKind::Global { prelude: true },
            None,
            Chain::default(),
            None,
        );
        for prelude in [false, true] {
            let id = ScopeId(index(engine.scopes.len()));
            engine.push_scope(
                ScopeKind::Global { prelude },
                None,
                Chain::default(),
                Some(id),
            );
            if prelude {
                engine.prelude_names_global = id;
            } else {
                engine.names_global = id;
            }
        }
        engine
    }

    pub fn analyze(&mut self) -> Result<(), Internal> {
        let outcome = self.stages();
        if !self.ids_exhausted {
            outcome?;
        }
        self.draw_declaration_cycles()?;
        self.flush_diagnostics();
        Ok(())
    }

    fn stages(&mut self) -> Result<(), Internal> {
        self.expand()?;
        self.count_inlining()?;
        if self.stop.is_some() {
            return Ok(());
        }
        self.infer()?;
        self.inferred = true;
        self.check_program()?;
        if self.stop.is_some() {
            return Ok(());
        }
        self.unexpanded_names(false)
    }

    pub(in crate::semantics::analysis) fn stop(
        &mut self,
        code: Code,
        loc: Loc,
        scope: Option<ScopeId>,
        (message, help): (String, Option<String>),
    ) -> Result<(), Internal> {
        if self.stop.is_some() {
            return Ok(());
        }
        self.draw_declaration_cycles()?;
        let related = match scope {
            Some(scope) => self
                .chain(scope)?
                .outermost_paths()
                .into_iter()
                .next()
                .unwrap_or_default(),
            None => Vec::new(),
        };
        self.stop = Some(super::stop::Stop {
            reported: self.report.diagnostics.len(),
            deferred: self.deferred.len(),
            inferred: self.inferred,
            code,
            loc,
            related,
            message,
            help,
        });
        Ok(())
    }

    pub fn demand(&mut self, query: Query) -> Result<(), Internal> {
        let mut stack: Vec<(Query, Option<usize>)> = vec![(query, None)];
        let mut nearest: Vec<Option<Query>> = vec![self.is_declaration(query)?.then_some(query)];
        let mut stalled = 0;
        while let Some((top, _)) = stack.last().copied() {
            crate::semantics::counted_walk::CountedWalk::EngineDemand.step();
            let position = stack.len() - 1;
            if self.is_done(top) {
                stack.pop();
                nearest.pop();
                self.waiting.remove(&top);
                self.settle_cycle_member(top);
                stalled = 0;
                continue;
            }
            let mut need = Vec::new();
            self.current = Some(top);
            let dispatched =
                self.dispatch_reader(&stack, position, nearest.get(position).copied().flatten())?;
            let enclosing_reader = std::mem::replace(&mut self.reader, dispatched);
            let done = self.dispatch(top, &mut need);
            self.reader = enclosing_reader;
            let done = done?;
            self.current = None;
            if done {
                stack.pop();
                nearest.pop();
                self.waiting.remove(&top);
                self.settle_cycle_member(top);
                stalled = 0;
                continue;
            }
            self.waiting.insert(top);
            let reader = nearest.get(position).copied().flatten();
            let mut progressed = false;
            for dep in need {
                let declares = self.is_declaration(dep)?;
                if declares && !matches!(top, Query::Eval(..)) {
                    self.note_read(reader, dep);
                }
                if self.waiting.contains(&dep) {
                    if let Some(cut) = self.candidate_cut(&stack, position, dep)? {
                        for (query, _) in stack.drain(cut + 1..) {
                            self.waiting.remove(&query);
                        }
                        nearest.truncate(cut + 1);
                        progressed = true;
                        break;
                    }
                    if self.cyclic.insert((top, dep)) {
                        self.cycle(&stack, position, dep)?;
                        progressed = true;
                    }
                } else if !self.is_done(dep) {
                    stack.push((dep, Some(position)));
                    let pushed = if declares {
                        Some(dep)
                    } else {
                        self.pushed_reader(dep, &stack, position, reader)?
                    };
                    nearest.push(pushed);
                    progressed = true;
                }
            }
            if progressed {
                stalled = 0;
            } else {
                stalled += 1;
                if stalled > 2 {
                    return Err(Internal::missed("query result"));
                }
            }
        }
        Ok(())
    }

    fn note_read(&self, reader: Option<Query>, dep: Query) {
        if matches!(self.current, Some(Query::Eval(..))) {
            return;
        }
        let reader = match dep {
            Query::Param(scope, _) => self.function_of_scope(scope).ok().flatten(),
            Query::Expr(..)
            | Query::Let(..)
            | Query::Input(_)
            | Query::State(_)
            | Query::Attr(..)
            | Query::Destination(_)
            | Query::Sig(_)
            | Query::Eval(..) => reader,
        };
        if let Some(reader) = reader
            && reader != dep
        {
            self.declaration_reads.borrow_mut().insert((reader, dep));
        }
    }

    fn function_of_scope(&self, scope: ScopeId) -> Result<Option<Query>, Internal> {
        let mut current = Some(scope);
        while let Some(at) = current {
            let record = self.scope_of(at)?;
            if let ScopeKind::Locals(locals) = &record.kind
                && let super::locals_owner::LocalsOwner::Fn(function) = locals.owner
            {
                return Ok(Some(Query::Expr(self.hir.fn_decl(function)?.value, at)));
            }
            current = match &record.kind {
                ScopeKind::ParamPrefix { locals, .. } => Some(*locals),
                ScopeKind::Global { .. }
                | ScopeKind::Locals(_)
                | ScopeKind::Body { .. }
                | ScopeKind::ForVar { .. }
                | ScopeKind::WireClause { .. }
                | ScopeKind::WrittenClause { .. } => record.parent,
            };
        }
        Ok(None)
    }

    fn pushed_reader(
        &self,
        dep: Query,
        stack: &[(Query, Option<usize>)],
        position: usize,
        reader: Option<Query>,
    ) -> Result<Option<Query>, Internal> {
        let (scope, loc, expression) = match dep {
            Query::Expr(expr, scope) => (scope, self.hir.expr(expr)?.loc, true),
            Query::Sig(node) | Query::Eval(node, _) => {
                let Some(record) = self.nodes.get(node.index()) else {
                    return Ok(reader);
                };
                (record.scope, record.loc, false)
            }
            Query::Let(..)
            | Query::Param(..)
            | Query::Input(_)
            | Query::State(_)
            | Query::Attr(..)
            | Query::Destination(_) => return Ok(reader),
        };
        if let Some(function) = self.function_of_scope(scope)? {
            let Query::Expr(_, locals) = function else {
                return Ok(reader);
            };
            if let ScopeKind::Locals(record) = &self.scope_of(locals)?.kind
                && let super::locals_owner::LocalsOwner::Fn(id) = record.owner
            {
                for (index, param) in self.hir.fn_decl(id)?.params.iter().enumerate() {
                    let Some(default) = param.default else {
                        continue;
                    };
                    let within = self.hir.expr(default)?.loc;
                    if within.file == loc.file && within.start <= loc.start && loc.end <= within.end
                    {
                        let index =
                            u32::try_from(index).map_err(|_| Internal::missed("parameter"))?;
                        return Ok(Some(Query::Param(locals, index)));
                    }
                }
            }
            let inside = match reader {
                Some(Query::Expr(_, at) | Query::Param(at, _) | Query::Let(at, _)) => {
                    self.function_of_scope(at)? == Some(function)
                }
                Some(
                    Query::Input(_)
                    | Query::State(_)
                    | Query::Attr(..)
                    | Query::Destination(_)
                    | Query::Sig(_)
                    | Query::Eval(..),
                )
                | None => false,
            };
            return Ok(if inside { reader } else { Some(function) });
        }
        let (true, Some(&(Query::Param(..), _))) = (expression, stack.get(position)) else {
            return Ok(reader);
        };
        self.outer_reader(stack, position)
    }

    fn outer_reader(
        &self,
        stack: &[(Query, Option<usize>)],
        position: usize,
    ) -> Result<Option<Query>, Internal> {
        let mut current = Some(position);
        while let Some(at) = current {
            let &(query, parent) = stack
                .get(at)
                .ok_or(Internal::missed("demand stack entry"))?;
            if matches!(
                query,
                Query::Let(..) | Query::Input(_) | Query::State(_) | Query::Attr(..)
            ) {
                return Ok(Some(query));
            }
            current = parent;
        }
        Ok(None)
    }

    fn dispatch_reader(
        &self,
        stack: &[(Query, Option<usize>)],
        position: usize,
        reader: Option<Query>,
    ) -> Result<Option<Query>, Internal> {
        let Some(&(Query::Param(locals, index), _)) = stack.get(position) else {
            return Ok(reader);
        };
        let ScopeKind::Locals(record) = &self.scope_of(locals)?.kind else {
            return Ok(reader);
        };
        let slot = usize::try_from(index).map_err(|_| Internal::missed("parameter"))?;
        match record.params.get(slot) {
            Some(super::param_source::ParamSource::Arg { scope, .. }) => {
                match self.function_of_scope(*scope)? {
                    Some(function) => Ok(Some(function)),
                    None => self.outer_reader(stack, position),
                }
            }
            Some(
                super::param_source::ParamSource::Default
                | super::param_source::ParamSource::Missing
                | super::param_source::ParamSource::Bound(_),
            )
            | None => Ok(reader),
        }
    }

    fn is_declaration(&self, query: Query) -> Result<bool, Internal> {
        Ok(match query {
            Query::Let(..)
            | Query::Param(..)
            | Query::Input(_)
            | Query::State(_)
            | Query::Attr(..) => true,
            Query::Expr(expr, scope) => match &self.scope_of(scope)?.kind {
                ScopeKind::Locals(locals) => match locals.owner {
                    super::locals_owner::LocalsOwner::Fn(function) => {
                        self.hir.fn_decl(function)?.value == expr
                    }
                    super::locals_owner::LocalsOwner::Pattern(_)
                    | super::locals_owner::LocalsOwner::For(_) => false,
                },
                ScopeKind::Global { .. }
                | ScopeKind::ParamPrefix { .. }
                | ScopeKind::Body { .. }
                | ScopeKind::ForVar { .. }
                | ScopeKind::WireClause { .. }
                | ScopeKind::WrittenClause { .. } => false,
            },
            Query::Destination(_) | Query::Sig(_) | Query::Eval(..) => false,
        })
    }

    pub fn in_progress(&self, query: Query) -> bool {
        self.current == Some(query) || self.waiting.contains(&query)
    }

    fn signing(&self) -> Option<NodeId> {
        let Some(Query::Sig(node)) = self.current else {
            return None;
        };
        Some(node)
    }

    pub fn declaring(&self) -> bool {
        self.current
            .iter()
            .chain(self.waiting.iter())
            .any(|query| !matches!(query, Query::Expr(..) | Query::Sig(_) | Query::Eval(..)))
    }

    fn is_done(&self, query: Query) -> bool {
        let memo = &self.memo;
        match query {
            Query::Expr(expr, scope) => memo.exprs.contains_key(&(expr, scope)),
            Query::Let(scope, local) => memo.lets.contains_key(&(scope, local)),
            Query::Param(scope, index) => memo.params.contains_key(&(scope, index)),
            Query::Input(input) => memo.inputs.contains_key(&input),
            Query::State(state) => memo.states.contains_key(&state),
            Query::Attr(item, attr) => memo.attrs.contains_key(&(item, attr)),
            Query::Destination(wire) => memo.destinations.contains_key(&wire),
            Query::Sig(node) => memo.sigs.contains_key(&node),
            Query::Eval(node, ty) => memo.evals.contains_key(&(node, ty)),
        }
    }

    #[cfg(not(test))]
    fn dispatch(&mut self, query: Query, need: &mut Vec<Query>) -> Result<bool, Internal> {
        self.run(query, need)
    }

    #[cfg(test)]
    fn dispatch(&mut self, query: Query, need: &mut Vec<Query>) -> Result<bool, Internal> {
        (self.run_query)(self, query, need)
    }

    fn run(&mut self, query: Query, need: &mut Vec<Query>) -> Result<bool, Internal> {
        self.memo.query_runs += 1;
        let sink = match query {
            Query::Eval(node, ty) => {
                super::sink::Sink::Into(super::reachable::Reachable::Eval(node, ty))
            }
            Query::Expr(..)
            | Query::Let(..)
            | Query::Param(..)
            | Query::Input(_)
            | Query::State(_)
            | Query::Attr(..)
            | Query::Destination(_)
            | Query::Sig(_) => super::sink::Sink::Reached,
        };
        let enclosing = std::mem::replace(&mut self.sink, sink);
        let ran = self.ran(query, need);
        self.sink = enclosing;
        ran
    }

    fn ran(&mut self, query: Query, need: &mut Vec<Query>) -> Result<bool, Internal> {
        match query {
            Query::Expr(expr, scope) => self.run_expr(expr, scope, need),
            Query::Let(scope, local) => self.run_let(scope, local, need),
            Query::Param(scope, index) => self.run_param(scope, index, need),
            Query::Input(input) => self.run_input(input, need),
            Query::State(state) => self.run_state(state, need),
            Query::Attr(item, attr) => self.run_attr(item, attr, need),
            Query::Destination(wire) => self.run_destination(wire, need),
            Query::Sig(node) => self.run_sig(node, need),
            Query::Eval(node, ty) => self.run_eval(node, ty, need),
        }
    }

    pub(super) fn poisoned(&self, dep: Query) -> bool {
        self.current
            .is_some_and(|current| self.cyclic.contains(&(current, dep)))
    }

    pub fn val(&self, query: Query, need: &mut Vec<Query>) -> Option<Val> {
        if self.poisoned(query) {
            return Some(Val::Error);
        }
        let Query::Expr(expr, scope) = query else {
            return Some(Val::Error);
        };
        let found = self.memo.exprs.get(&(expr, scope)).cloned();
        if found.is_none() {
            need.push(query);
        }
        found
    }

    pub fn bound(&self, query: Query, need: &mut Vec<Query>) -> Option<BoundVal> {
        self.note_read(self.reader, query);
        if self.poisoned(query) || self.cycle_members.contains(&query) {
            return Some(BoundVal::Error);
        }
        let memo = &self.memo;
        let found = match query {
            Query::Let(scope, local) => memo.lets.get(&(scope, local)).cloned(),
            Query::Param(scope, index) => memo.params.get(&(scope, index)).cloned(),
            Query::Input(input) => memo.inputs.get(&input).cloned(),
            Query::State(state) => memo.states.get(&state).cloned(),
            Query::Attr(item, attr) => memo.attrs.get(&(item, attr)).cloned(),
            Query::Expr(..) | Query::Destination(_) | Query::Sig(_) | Query::Eval(..) => {
                Some(BoundVal::Error)
            }
        };
        if found.is_none() {
            need.push(query);
        }
        found
    }

    pub fn destination(&self, query: Query, need: &mut Vec<Query>) -> Option<Destination> {
        if self.poisoned(query) {
            return Some(Destination::Error);
        }
        let Query::Destination(wire) = query else {
            return Some(Destination::Error);
        };
        let found = self.memo.destinations.get(&wire).cloned();
        if found.is_none() {
            need.push(query);
        }
        found
    }

    pub fn sig(&self, node: NodeId, need: &mut Vec<Query>) -> Option<Sig> {
        let query = Query::Sig(node);
        if self.poisoned(query) {
            return Some(Sig::ERROR);
        }
        let found = self.memo.sigs.get(&node).copied();
        if found.is_none() {
            need.push(query);
        }
        found
    }

    pub fn known_sig(&self, node: NodeId) -> Result<Sig, Internal> {
        self.memo
            .sigs
            .get(&node)
            .copied()
            .ok_or(Internal::missed("signature"))
    }

    pub fn cval(&mut self, query: Query, need: &mut Vec<Query>) -> Option<CVal> {
        if let Query::Eval(node, ty) = query {
            let sink = self.sink;
            self.link(&sink, super::reachable::Reachable::Eval(node, ty));
        }
        self.untaken_cval(query, need)
    }

    pub fn untaken_cval(&self, query: Query, need: &mut Vec<Query>) -> Option<CVal> {
        if self.poisoned(query) {
            return Some(CVal::Error);
        }
        let Query::Eval(node, ty) = query else {
            return Some(CVal::Error);
        };
        let found = self.memo.evals.get(&(node, ty)).cloned();
        if found.is_none() {
            need.push(query);
        }
        found
    }

    pub fn demand_val(&mut self, query: Query) -> Result<Val, Internal> {
        self.demand(query)?;
        let mut need = Vec::new();
        self.val(query, &mut need)
            .ok_or(Internal::missed("demanded value"))
    }

    pub fn demand_sig(&mut self, node: NodeId) -> Result<Sig, Internal> {
        self.demand(Query::Sig(node))?;
        self.memo
            .sigs
            .get(&node)
            .copied()
            .ok_or(Internal::missed("demanded signature"))
    }

    pub fn demand_cval(
        &mut self,
        node: NodeId,
        ty: super::eval_ty::EvalTy,
    ) -> Result<CVal, Internal> {
        let sink = self.sink;
        self.link(&sink, super::reachable::Reachable::Eval(node, ty));
        self.demand(Query::Eval(node, ty))?;
        self.memo
            .evals
            .get(&(node, ty))
            .cloned()
            .ok_or(Internal::missed("demanded compile-time value"))
    }

    pub fn scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<ScopeId>,
        chain: Chain,
    ) -> Result<ScopeId, Internal> {
        let template = match parent {
            Some(parent) => self.scope_of(parent)?.template,
            None => None,
        };
        self.scope_in(kind, parent, chain, template)
    }

    pub fn scope_in(
        &mut self,
        kind: ScopeKind,
        parent: Option<ScopeId>,
        chain: Chain,
        template: Option<ScopeId>,
    ) -> Result<ScopeId, Internal> {
        let loc = self.nodes.last().map_or(Loc::default(), |node| node.loc);
        self.claim_id(self.scopes.len(), "scopes", loc)?;
        Ok(self.push_scope(kind, parent, chain, template))
    }

    fn push_scope(
        &mut self,
        kind: ScopeKind,
        parent: Option<ScopeId>,
        chain: Chain,
        template: Option<ScopeId>,
    ) -> ScopeId {
        let id = ScopeId(index(self.scopes.len()));
        self.scopes.push(Scope {
            kind,
            parent,
            chain,
            template,
        });
        id
    }

    pub fn template_of(&self, scope: ScopeId) -> Result<Option<ScopeId>, Internal> {
        Ok(self.scope_of(scope)?.template)
    }

    pub fn scope_of(&self, id: ScopeId) -> Result<&Scope, Internal> {
        self.scopes.get(id.index()).ok_or(Internal::missed("scope"))
    }

    pub fn node(&self, id: NodeId) -> Result<&Node, Internal> {
        self.nodes.get(id.index()).ok_or(Internal::missed("node"))
    }

    pub fn slot(&self, id: super::ids::SlotId) -> Result<&super::slot::Slot, Internal> {
        self.slots
            .get(id.index())
            .ok_or(Internal::missed("loop slot"))
    }

    pub fn binding(&self, id: BindingId) -> Result<&Binding, Internal> {
        self.bindings
            .get(id.index())
            .ok_or(Internal::missed("binding"))
    }

    pub fn node_decl(&self, kind: NodeKind) -> Result<&'a NodeKindDecl, Internal> {
        self.vocabulary
            .node_kind(kind)
            .ok_or(Internal::missed("node kind"))
    }

    pub fn wire_decl(&self, kind: WireKind) -> Result<&'a WireKindDecl, Internal> {
        self.vocabulary
            .wire_kind(kind)
            .ok_or(Internal::missed("wire kind"))
    }

    pub fn enum_decl(&self, id: EnumId) -> Result<&'a EnumDecl, Internal> {
        self.vocabulary
            .enum_decl(id)
            .ok_or(Internal::missed("enum"))
    }

    pub fn family_decl(&self, id: FamilyId) -> Result<&'a FamilyDecl, Internal> {
        self.vocabulary
            .family(id)
            .ok_or(Internal::missed("unit family"))
    }

    pub fn unit_decl(&self, family: FamilyId, unit: usize) -> Result<&'a UnitDecl, Internal> {
        self.family_decl(family)?
            .units
            .get(unit)
            .ok_or(Internal::missed("unit"))
    }

    pub fn push_node(&mut self, op: Op, loc: Loc, scope: ScopeId) -> Result<NodeId, Internal> {
        self.claim_id(self.nodes.len(), "values", loc)?;
        let id = NodeId(index(self.nodes.len()));
        match self.dimensions_of(&op)? {
            Dimensions::Known(0) => {}
            Dimensions::Known(known) => {
                self.arrays.insert(id);
                self.memo.dimensions.insert(id, known);
            }
            Dimensions::Unknown => {
                self.arrays.insert(id);
            }
        }
        self.nodes.push(Node { op, loc, scope });
        Ok(id)
    }

    pub fn holds_array(&self, val: &Val) -> bool {
        matches!(val, Val::Scalar(node) if self.arrays.contains(node))
    }

    pub fn push_binding(&mut self, binding: Binding) -> Result<BindingId, Internal> {
        self.claim_id(self.bindings.len(), "cells", binding.loc)?;
        let id = BindingId(index(self.bindings.len()));
        self.bindings.push(binding);
        Ok(id)
    }

    pub(in crate::semantics::analysis) fn claim_id(
        &mut self,
        taken: usize,
        space: &str,
        loc: Loc,
    ) -> Result<(), Internal> {
        if u64::try_from(taken).is_ok_and(|taken| taken < self.id_space) {
            return Ok(());
        }
        self.ids_exhausted = true;
        let message = format!(
            "the compilation needs more than {} {space}, the most its ids number",
            grouped(self.id_space)
        );
        self.stop(Code::ExpansionLimit, loc, None, (message, None))?;
        Err(Internal::missed("id"))
    }

    pub fn chain(&self, scope: ScopeId) -> Result<Chain, Internal> {
        Ok(self.scope_of(scope)?.chain.clone())
    }

    pub fn error(
        &mut self,
        code: Code,
        loc: Loc,
        scope: ScopeId,
        message: impl Into<String>,
    ) -> Result<(), Internal> {
        if self.suppressed(code, scope)? {
            return Ok(());
        }
        self.error_with_chain(code, loc, Vec::new(), scope, message)
    }

    pub(in crate::semantics::analysis) fn error_helped(
        &mut self,
        code: Code,
        loc: Loc,
        scope: ScopeId,
        message: String,
        help: String,
    ) -> Result<(), Internal> {
        if self.suppressed(code, scope)? {
            return Ok(());
        }
        self.chained(code, loc, Vec::new(), scope, (message, Some(help)))
    }

    pub(super) fn in_probe(&self, scope: ScopeId) -> Result<bool, Internal> {
        Ok(self.in_trial(scope)? || self.in_name_probe(scope)?)
    }

    pub(super) fn in_trial(&self, scope: ScopeId) -> Result<bool, Internal> {
        Ok(self
            .template_of(scope)?
            .is_some_and(|template| self.memo.probes.contains(&template)))
    }

    pub(super) fn in_name_probe(&self, scope: ScopeId) -> Result<bool, Internal> {
        Ok(self.template_of(scope)?.is_some_and(|template| {
            template == self.names_global || template == self.prelude_names_global
        }))
    }

    pub(super) fn names_global_of(&self, global: ScopeId) -> ScopeId {
        if global == self.prelude_global {
            self.prelude_names_global
        } else {
            self.names_global
        }
    }

    fn suppressed(&self, code: Code, scope: ScopeId) -> Result<bool, Internal> {
        if self.in_name_probe(scope)? && !self.in_trial(scope)? {
            return Ok(!NAME_CODES.contains(&code));
        }
        self.in_trial(scope)
    }

    pub fn error_with_chain(
        &mut self,
        code: Code,
        loc: Loc,
        before: Vec<(Loc, String)>,
        scope: ScopeId,
        message: impl Into<String>,
    ) -> Result<(), Internal> {
        if self.suppressed(code, scope)? {
            return Ok(());
        }
        self.chained(code, loc, before, scope, (message.into(), None))
    }

    fn chained(
        &mut self,
        code: Code,
        loc: Loc,
        before: Vec<(Loc, String)>,
        scope: ScopeId,
        (message, help): (String, Option<String>),
    ) -> Result<(), Internal> {
        let chain = self.chain(scope)?;
        let (fault, template) = self.origin(scope)?;
        let deferred = super::deferred::Deferred {
            code,
            loc,
            before,
            chain,
            message,
            help,
            unless_untyped: Vec::new(),
            unless_values: Vec::new(),
            fault,
            template,
            signing: self.signing(),
            node: None,
        };
        self.place(deferred, VALUE_CODES.contains(&code))
    }

    pub(in crate::semantics::analysis) fn place(
        &mut self,
        deferred: super::deferred::Deferred,
        value: bool,
    ) -> Result<(), Internal> {
        if !value {
            self.deferred.push(deferred);
            return Ok(());
        }
        match self.sink {
            super::sink::Sink::Reached => self.deferred.push(deferred),
            super::sink::Sink::Discard => {}
            super::sink::Sink::Into(target) => {
                let state = self.memo.recorded.entry(target).or_insert_with(|| {
                    super::fault_state::FaultState::Recorded {
                        faults: Vec::new(),
                        edges: Vec::new(),
                    }
                });
                match state {
                    super::fault_state::FaultState::Recorded { faults, .. } => {
                        faults.push(deferred);
                    }
                    super::fault_state::FaultState::Drawn => self.deferred.push(deferred),
                }
            }
        }
        Ok(())
    }

    pub(in crate::semantics::analysis) fn link(
        &mut self,
        from: &super::sink::Sink,
        to: super::reachable::Reachable,
    ) {
        let settled = match self.memo.recorded.get(&to) {
            Some(super::fault_state::FaultState::Drawn) => true,
            Some(super::fault_state::FaultState::Recorded { .. }) => false,
            None => self.complete(&to),
        };
        if !settled {
            self.edge(from, to);
        }
    }

    pub(in crate::semantics::analysis) fn edge(
        &mut self,
        from: &super::sink::Sink,
        to: super::reachable::Reachable,
    ) {
        let target = match from {
            super::sink::Sink::Reached => {
                self.draw_reached(to);
                return;
            }
            super::sink::Sink::Discard => return,
            super::sink::Sink::Into(target) => *target,
        };
        if target == to {
            return;
        }
        let state = self.memo.recorded.entry(target).or_insert_with(|| {
            super::fault_state::FaultState::Recorded {
                faults: Vec::new(),
                edges: Vec::new(),
            }
        });
        match state {
            super::fault_state::FaultState::Recorded { edges, .. } => edges.push(to),
            super::fault_state::FaultState::Drawn => self.draw_reached(to),
        }
    }

    fn complete(&self, reachable: &super::reachable::Reachable) -> bool {
        match reachable {
            super::reachable::Reachable::Eval(node, ty) => {
                self.memo.evals.contains_key(&(*node, *ty))
            }
            super::reachable::Reachable::Call(id) => self.memo.calls.contains_key(id),
            super::reachable::Reachable::Loop(..) => false,
        }
    }

    pub(in crate::semantics::analysis) fn draw_reached(
        &mut self,
        first: super::reachable::Reachable,
    ) {
        let mut pending = vec![first];
        while let Some(current) = pending.pop() {
            crate::semantics::counted_walk::CountedWalk::EngineDrawReached.step();
            match self
                .memo
                .recorded
                .insert(current, super::fault_state::FaultState::Drawn)
            {
                Some(super::fault_state::FaultState::Recorded { faults, edges }) => {
                    self.deferred.extend(faults);
                    pending.extend(edges);
                }
                Some(super::fault_state::FaultState::Drawn) | None => {}
            }
        }
    }

    pub(in crate::semantics::analysis) fn holds_faults(
        &self,
        first: &super::reachable::Reachable,
    ) -> bool {
        let mut seen: FastSet<&super::reachable::Reachable> = FastSet::default();
        let mut pending = vec![first];
        while let Some(current) = pending.pop() {
            crate::semantics::counted_walk::CountedWalk::EngineHoldsFaults.step();
            if !seen.insert(current) {
                continue;
            }
            match self.memo.recorded.get(current) {
                Some(super::fault_state::FaultState::Recorded { faults, edges }) => {
                    if !faults.is_empty() {
                        return true;
                    }
                    pending.extend(edges.iter());
                }
                Some(super::fault_state::FaultState::Drawn) | None => {}
            }
        }
        false
    }

    fn origin(&mut self, scope: ScopeId) -> Result<(Option<u32>, Option<ScopeId>), Internal> {
        let Some(template) = self.template_of(scope)? else {
            return Ok((None, None));
        };
        if self.in_probe(scope)? {
            return Ok((None, None));
        }
        let root = match (self.fault_frame, self.active.as_ref()) {
            (Some(frame), Some(machine)) => machine.frames.get(frame).map(|record| record.root),
            _ => None,
        };
        let Some(root) = root else {
            return Ok((None, Some(template)));
        };
        if self.template_of(self.node(root)?.scope)? != Some(template) {
            return Ok((None, Some(template)));
        }
        let fault = self.fault_count;
        self.fault_count += 1;
        if let Some(record) = self.active.as_mut().and_then(|machine| {
            self.fault_frame
                .and_then(|frame| machine.frames.get_mut(frame))
        }) {
            record.faults.insert(fault);
        }
        Ok((Some(fault), Some(template)))
    }

    pub fn defer(
        &mut self,
        code: Code,
        loc: Loc,
        before: Vec<(Loc, String)>,
        chain: Chain,
        message: impl Into<String>,
    ) {
        self.deferred.push(super::deferred::Deferred {
            code,
            loc,
            before,
            chain,
            message: message.into(),
            help: None,
            unless_untyped: Vec::new(),
            unless_values: Vec::new(),
            fault: None,
            template: None,
            signing: self.signing(),
            node: None,
        });
    }

    fn attributed_chains(&self, deferred: &super::deferred::Deferred) -> Vec<Chain> {
        let unchanged = || vec![deferred.chain.clone()];
        let Some(template) = deferred.template else {
            return unchanged();
        };
        let sites = match deferred.fault {
            Some(fault) => self
                .memo
                .fault_sites
                .get(&fault)
                .cloned()
                .unwrap_or_default(),
            None => self.template_sites(template),
        };
        let Some(base) = self.memo.template_base.get(&template) else {
            return unchanged();
        };
        let inner = deferred.chain.depth().saturating_sub(*base);
        let chains: Vec<Chain> = sites
            .iter()
            .filter_map(|site| {
                let node = self.nodes.get(site.index())?;
                let scope = self.scopes.get(node.scope.index())?;
                Some(
                    deferred
                        .chain
                        .rebase(inner, &scope.chain.push(node.loc, "in this call")),
                )
            })
            .collect();
        if chains.is_empty() {
            return unchanged();
        }
        chains
    }

    fn template_sites(&self, template: ScopeId) -> Vec<NodeId> {
        let mut sites = Vec::new();
        let mut seen = super::fast_hash::FastSet::default();
        let mut pending = vec![template];
        while let Some(current) = pending.pop() {
            crate::semantics::counted_walk::CountedWalk::EngineTemplateSites.step();
            if !seen.insert(current) {
                continue;
            }
            if let Some(direct) = self.memo.template_sites.get(&current) {
                sites.extend(direct.iter().copied());
            }
            if let Some(parents) = self.memo.template_parents.get(&current) {
                pending.extend(parents.iter().copied());
            }
        }
        sites
    }

    pub(in crate::semantics::analysis) fn reach_eager_starts(&mut self) {
        let mut starts: Vec<((ExprId, ScopeId), NodeId)> = self
            .memo
            .eager_starts
            .iter()
            .filter(|(_, (_, passed))| !passed)
            .map(|(at, (start, _))| (*at, *start))
            .collect();
        starts.sort_by_key(|(_, start)| start.index());
        for (at, start) in starts {
            self.memo.eager_starts.remove(&at);
            self.draw_reached(super::reachable::Reachable::Eval(
                start,
                super::eval_ty::EvalTy::Node,
            ));
        }
    }

    pub fn flush_diagnostics(&mut self) {
        self.reach_eager_starts();
        let stop = self.stop.take();
        if let Some(stop) = &stop {
            self.report.diagnostics.truncate(stop.reported);
            self.deferred.truncate(stop.deferred);
            if !stop.inferred {
                self.deferred.retain(|deferred| {
                    deferred.unless_untyped.is_empty() && deferred.unless_values.is_empty()
                });
            }
        }
        let mut merged: FastMap<Merged, usize> = FastMap::default();
        let mut chosen: Vec<Chosen> = Vec::new();
        for deferred in std::mem::take(&mut self.deferred) {
            if deferred
                .template
                .is_some_and(|template| self.memo.probes.contains(&template))
                || deferred
                    .unless_untyped
                    .iter()
                    .any(|binding| self.typing.untyped.contains(binding))
                || deferred
                    .unless_values
                    .iter()
                    .any(|value| self.typing.untyped_values.contains(value))
            {
                continue;
            }
            for chain in self.attributed_chains(&deferred) {
                let depth = chain.depth();
                for path in chain.outermost_paths() {
                    let key = Merged {
                        code: deferred.code,
                        loc: deferred.loc,
                        before: deferred.before.clone(),
                        message: deferred.message.clone(),
                        outermost: path.last().map(|(loc, _)| *loc),
                    };
                    let mut related = deferred.before.clone();
                    related.extend(path);
                    match merged.get(&key).and_then(|index| chosen.get_mut(*index)) {
                        Some(earlier) if earlier.4 < depth => {
                            earlier.2 = related;
                            earlier.4 = depth;
                        }
                        Some(_) => {}
                        None => {
                            merged.insert(key, chosen.len());
                            chosen.push((
                                deferred.code,
                                deferred.loc,
                                related,
                                (deferred.message.clone(), deferred.help.clone()),
                                depth,
                            ));
                        }
                    }
                }
            }
        }
        for (code, loc, related, (message, help), _) in chosen {
            match help {
                Some(help) => self
                    .report
                    .hinted(self.hir, code, loc, &related, message, help),
                None => self
                    .report
                    .error_with(self.hir, code, loc, &related, message),
            }
        }
        if let Some(stop) = stop {
            match stop.help {
                Some(help) => self.report.hinted(
                    self.hir,
                    stop.code,
                    stop.loc,
                    &stop.related,
                    stop.message,
                    help,
                ),
                None => self.report.error_with(
                    self.hir,
                    stop.code,
                    stop.loc,
                    &stop.related,
                    stop.message,
                ),
            }
        }
    }

    pub fn error_at_node(
        &mut self,
        code: Code,
        node: NodeId,
        message: impl Into<String>,
    ) -> Result<(), Internal> {
        let value = VALUE_CODES.contains(&code);
        self.placed_at_node(code, node, Vec::new(), (message.into(), None), value)
    }

    pub(in crate::semantics::analysis) fn index_outside(
        &mut self,
        index: NodeId,
        position: i128,
        length: u64,
    ) -> Result<(), Internal> {
        let message = format!("the index {position} is outside the array of length {length}");
        match crate::diagnostics::index_outside_help(length) {
            Some(help) => {
                self.error_at_node_helped(Code::IndexOutOfRange, index, Vec::new(), message, help)
            }
            None => self.error_at_node(Code::IndexOutOfRange, index, message),
        }
    }

    pub(in crate::semantics::analysis) fn error_at_node_helped(
        &mut self,
        code: Code,
        node: NodeId,
        before: Vec<(Loc, String)>,
        message: String,
        help: String,
    ) -> Result<(), Internal> {
        let value = VALUE_CODES.contains(&code);
        self.placed_at_node(code, node, before, (message, Some(help)), value)
    }

    pub(in crate::semantics::analysis) fn placed_at_node(
        &mut self,
        code: Code,
        node: NodeId,
        before: Vec<(Loc, String)>,
        (message, help): (String, Option<String>),
        value: bool,
    ) -> Result<(), Internal> {
        let (loc, scope) = {
            let record = self.node(node)?;
            (record.loc, record.scope)
        };
        if self.suppressed(code, scope)? {
            return Ok(());
        }
        let unless_untyped = self.untypable_reads(node)?;
        let unless_values = self.untypable_values(node)?;
        let chain = self.chain(scope)?;
        let (fault, template) = self.origin(scope)?;
        let deferred = super::deferred::Deferred {
            code,
            loc,
            before,
            chain,
            message,
            help,
            unless_untyped,
            unless_values,
            fault,
            template,
            signing: self.signing(),
            node: Some(node),
        };
        self.place(deferred, value)
    }

    pub fn error_unless_values(
        &mut self,
        code: Code,
        loc: Loc,
        scope: ScopeId,
        message: impl Into<String>,
        help: Option<String>,
        unless_values: Vec<NodeId>,
    ) -> Result<(), Internal> {
        if self.suppressed(code, scope)? {
            return Ok(());
        }
        let chain = self.chain(scope)?;
        let (fault, template) = self.origin(scope)?;
        let deferred = super::deferred::Deferred {
            code,
            loc,
            before: Vec::new(),
            chain,
            message: message.into(),
            help,
            unless_untyped: Vec::new(),
            unless_values,
            fault,
            template,
            signing: self.signing(),
            node: None,
        };
        self.place(deferred, VALUE_CODES.contains(&code))
    }

    pub fn read(&mut self, bound: &BoundVal, loc: Loc, scope: ScopeId) -> Result<Val, Internal> {
        Ok(match bound {
            BoundVal::Scalar(item) => Val::Scalar(self.read_one(*item, loc, scope)?),
            BoundVal::Array(array) => {
                let (shape, items) = array.as_ref();
                let elements = items
                    .iter()
                    .map(|item| self.read_one(*item, loc, scope))
                    .collect::<Result<_, _>>()?;
                Val::array(shape.clone(), elements)
            }
            BoundVal::Error => Val::Error,
        })
    }

    pub(in crate::semantics::analysis) fn read_one(
        &mut self,
        item: Bound,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<NodeId, Internal> {
        match item {
            Bound::Cell(binding) => self.push_node(Op::Read(binding), loc, scope),
            Bound::Value(node) => self.push_node(Op::Use(node), loc, scope),
        }
    }

    fn candidate_cut(
        &mut self,
        stack: &[(Query, Option<usize>)],
        top: usize,
        dep: Query,
    ) -> Result<Option<usize>, Internal> {
        let mut lower = dep;
        let mut position = top;
        loop {
            let &(query, parent) = stack
                .get(position)
                .ok_or(Internal::missed("demand stack entry"))?;
            if let Some(key) = self.candidate_edges.get(&(query, lower)).cloned() {
                let (path, _) = self.demand_path(stack, top, dep)?;
                self.open_candidates.insert(key, path.into());
                return Ok(Some(position));
            }
            if query == dep {
                return Ok(None);
            }
            lower = query;
            position = parent.ok_or(Internal::missed("cycle dependency"))?;
        }
    }

    fn demand_path(
        &self,
        stack: &[(Query, Option<usize>)],
        top: usize,
        dep: Query,
    ) -> Result<(Vec<Query>, Option<usize>), Internal> {
        let mut path = Vec::new();
        let mut current = Some(top);
        loop {
            let position = current.ok_or(Internal::missed("cycle dependency"))?;
            let &(query, parent) = stack
                .get(position)
                .ok_or(Internal::missed("demand stack entry"))?;
            path.push(query);
            current = parent;
            if query == dep {
                break;
            }
        }
        path.reverse();
        Ok((path, current))
    }

    pub(super) fn cycle(
        &mut self,
        stack: &[(Query, Option<usize>)],
        top: usize,
        dep: Query,
    ) -> Result<(), Internal> {
        let (path, mut current) = self.demand_path(stack, top, dep)?;
        let through_local_let = path
            .iter()
            .any(|query| matches!(query, Query::Let(_, LetRef::Local(_))));
        let through_top_level = path.iter().any(|query| {
            matches!(
                query,
                Query::Let(_, LetRef::Top(_)) | Query::Input(_) | Query::State(_) | Query::Attr(..)
            )
        });
        if through_local_let && !through_top_level {
            return Ok(());
        }
        let mut names = Vec::new();
        let mut locs = Vec::new();
        for query in &path {
            if let Query::Let(scope, local) = *query
                && let Some(decl) = self.let_decl(scope, local)?
            {
                let span = self.hir.expr(decl.value)?.loc;
                self.cycle_spans.push(span);
            }
        }
        for query in &path {
            if let Some((name, loc)) = self.declaration(*query)? {
                names.push(name);
                locs.push(loc);
            }
        }
        if names.is_empty() && self.evaluated_cycle(&path)? {
            return Ok(());
        }
        if names.is_empty() {
            let mut outer = None;
            while let Some(position) = current {
                crate::semantics::counted_walk::CountedWalk::EngineCycle.step();
                let &(query, parent) = stack
                    .get(position)
                    .ok_or(Internal::missed("demand stack entry"))?;
                if let Query::Eval(node, _) = query
                    && matches!(self.node(node)?.op, Op::Recursive { .. })
                {
                    outer = Some(node);
                }
                current = parent;
            }
            return self.endless_evaluation(&path, outer);
        }
        self.cycle_members
            .extend(path.iter().copied().filter(|query| {
                matches!(
                    query,
                    Query::Let(..)
                        | Query::Param(..)
                        | Query::Input(_)
                        | Query::State(_)
                        | Query::Attr(..)
                )
            }));
        self.declaration_cycle(&path, names, &locs)
    }

    pub(super) fn live_candidate_cycle(
        &mut self,
        item: ItemRef,
        name: &str,
        path: &[Query],
    ) -> Result<bool, Internal> {
        if !self.cycle_evaluating(path)? {
            return Ok(false);
        }
        if !self.untaken()? && self.drawn_candidate_cycles.insert((item, name.to_string())) {
            self.draw_candidate_cycle(path)?;
        }
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn note_value_binding(
        &mut self,
        query: Query,
        bound: &BoundVal,
    ) {
        let nodes: Vec<NodeId> = match bound {
            BoundVal::Scalar(Bound::Value(node)) => vec![*node],
            BoundVal::Array(array) => array
                .1
                .iter()
                .filter_map(|item| match item {
                    Bound::Value(node) => Some(*node),
                    Bound::Cell(_) => None,
                })
                .collect(),
            BoundVal::Scalar(Bound::Cell(_)) | BoundVal::Error => Vec::new(),
        };
        for node in nodes {
            self.memo.value_bindings.entry(node).or_insert(query);
        }
    }

    pub(in crate::semantics::analysis) fn evaluating_path(
        &self,
        start: NodeId,
    ) -> Result<Option<Vec<NodeId>>, Internal> {
        let evaluating: FastSet<NodeId> = self
            .current
            .iter()
            .chain(self.waiting.iter())
            .filter_map(|query| match query {
                Query::Eval(node, _) => Some(*node),
                Query::Expr(..)
                | Query::Let(..)
                | Query::Param(..)
                | Query::Input(_)
                | Query::State(_)
                | Query::Attr(..)
                | Query::Destination(_)
                | Query::Sig(_) => None,
            })
            .collect();
        if evaluating.is_empty() {
            return Ok(None);
        }
        let mut parents: FastMap<NodeId, NodeId> = FastMap::default();
        let mut seen: FastSet<NodeId> = FastSet::default();
        seen.insert(start);
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::EngineEvaluatingPath.step();
            if evaluating.contains(&node) {
                let mut path = vec![node];
                let mut current = node;
                while let Some(parent) = parents.get(&current) {
                    path.push(*parent);
                    current = *parent;
                }
                path.reverse();
                return Ok(Some(path));
            }
            for operand in self.node(node)?.op.operands() {
                if seen.insert(operand) {
                    parents.insert(operand, node);
                    stack.push(operand);
                }
            }
        }
        Ok(None)
    }

    pub(in crate::semantics::analysis) fn draw_evaluation_cycle(
        &mut self,
        (item, attr): (ItemRef, symphonyscript_core::AttrId),
        steps: Vec<Query>,
    ) -> Result<bool, Internal> {
        let mut queries: Vec<Query> = Vec::with_capacity(steps.len());
        for query in steps {
            if queries.last() != Some(&query) {
                queries.push(query);
            }
        }
        while queries.len() > 1 && queries.first() == queries.last() {
            queries.pop();
        }
        let mut names = Vec::new();
        let mut locs = Vec::new();
        for query in &queries {
            if let Some((name, loc)) = self.declaration(*query)? {
                names.push(name);
                locs.push(loc);
            }
        }
        if names.is_empty() {
            return Ok(false);
        }
        let attribute = self
            .attr_decl(item, attr)?
            .map_or(String::new(), |decl| decl.name.clone());
        if self.drawn_candidate_cycles.insert((item, attribute)) {
            self.declaration_cycle(&queries, names, &locs)?;
        }
        Ok(true)
    }

    fn function_query(&self, scope: ScopeId) -> Result<Option<Query>, Internal> {
        let Some(template) = self.template_of(scope)? else {
            return Ok(None);
        };
        let ScopeKind::Locals(locals) = &self.scope_of(template)?.kind else {
            return Ok(None);
        };
        let super::locals_owner::LocalsOwner::Fn(function) = locals.owner else {
            return Ok(None);
        };
        Ok(Some(Query::Expr(
            self.hir.fn_decl(function)?.value,
            template,
        )))
    }

    fn evaluated_cycle(&mut self, path: &[Query]) -> Result<bool, Internal> {
        let mut read = None;
        let mut steps = Vec::new();
        for query in path {
            let Query::Eval(node, _) = *query else {
                continue;
            };
            if let Some(&(item, attr)) = self.memo.attr_reads.get(&node) {
                if let Some(function) = self.function_query(self.node(node)?.scope)? {
                    steps.push(function);
                }
                steps.push(Query::Attr(item, attr));
                read.get_or_insert((item, attr));
            } else if let Some(binding) = self.memo.value_bindings.get(&node) {
                steps.push(*binding);
            }
        }
        match read {
            Some(read) => self.draw_evaluation_cycle(read, steps),
            None => Ok(false),
        }
    }

    fn cycle_evaluating(&self, path: &[Query]) -> Result<bool, Internal> {
        let mut values = Vec::new();
        for query in path {
            let value = match *query {
                Query::Attr(item, attr) => match self.memo.attrs.get(&(item, attr)) {
                    Some(BoundVal::Scalar(Bound::Value(node))) => Some(*node),
                    Some(
                        BoundVal::Scalar(Bound::Cell(_)) | BoundVal::Array(_) | BoundVal::Error,
                    )
                    | None => None,
                },
                Query::Let(scope, local) => match self.memo.lets.get(&(scope, local)) {
                    Some(BoundVal::Scalar(Bound::Value(node))) => Some(*node),
                    Some(
                        BoundVal::Scalar(Bound::Cell(_)) | BoundVal::Array(_) | BoundVal::Error,
                    )
                    | None => None,
                },
                Query::Expr(expr, scope) => match self.memo.exprs.get(&(expr, scope)) {
                    Some(Val::Scalar(node)) => Some(*node),
                    Some(Val::Array(_) | Val::Error) | None => None,
                },
                Query::Sig(node) => Some(node),
                Query::Param(..)
                | Query::Input(_)
                | Query::State(_)
                | Query::Destination(_)
                | Query::Eval(..) => None,
            };
            if let Some(node) = value {
                values.push(node);
                values.push(self.through_uses(node)?);
            }
        }
        Ok(self
            .current
            .iter()
            .chain(self.waiting.iter())
            .any(|query| matches!(query, Query::Eval(node, _) if values.contains(node))))
    }

    fn draw_candidate_cycle(&mut self, path: &[Query]) -> Result<(), Internal> {
        let mut names = Vec::new();
        let mut locs = Vec::new();
        for query in path {
            if let Some((name, loc)) = self.declaration(*query)? {
                names.push(name);
                locs.push(loc);
            }
        }
        if names.is_empty() {
            return Ok(());
        }
        self.declaration_cycle(path, names, &locs)
    }

    fn declaration_cycle(
        &mut self,
        path: &[Query],
        names: Vec<String>,
        locs: &[Loc],
    ) -> Result<(), Internal> {
        if names.is_empty() {
            return Err(Internal::missed("cycle declaration"));
        }
        let reported = path.iter().any(|query| {
            matches!(query, Query::Let(scope, LetRef::Top(id))
                if *scope == self.source_global && self.memo.cyclic_lets.contains(id))
        });
        if reported {
            return Ok(());
        }
        self.found_cycles
            .push(locs.iter().copied().zip(names).collect());
        Ok(())
    }

    fn endless_evaluation(
        &mut self,
        path: &[Query],
        outer: Option<NodeId>,
    ) -> Result<(), Internal> {
        let mut calls = Vec::new();
        for query in path {
            if let Query::Eval(node, _) = *query
                && let Op::Recursive { function, .. } = self.node(node)?.op
            {
                calls.push((node, function));
            }
        }
        let &(first, _) = calls.first().ok_or(Internal::missed("cycle declaration"))?;
        let loc = self.node(first)?.loc;
        let mut names: Vec<String> = Vec::with_capacity(calls.len());
        for (_, function) in calls {
            let name = self.function_name(function)?;
            if !names.contains(&name) {
                names.push(name);
            }
        }
        let message = format!(
            "the evaluation goes deeper than {} nested calls: {}",
            grouped(self.limits.nesting_depth),
            names.join(" → ")
        );
        let mut related = Vec::new();
        if let Some(outer) = outer {
            let (loc, scope) = {
                let record = self.node(outer)?;
                (record.loc, record.scope)
            };
            related.push((loc, "the outermost call the evaluation ends".to_string()));
            related.extend(
                self.chain(scope)?
                    .outermost_paths()
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
            );
        }
        self.report
            .error_with(self.hir, Code::RecursionDepth, loc, &related, message);
        Ok(())
    }

    fn settle_cycle_member(&mut self, query: Query) {
        if !self.cycle_members.remove(&query) {
            return;
        }
        let memo = &mut self.memo;
        match query {
            Query::Let(scope, local) => {
                memo.lets.insert((scope, local), BoundVal::Error);
                if let LetRef::Top(id) = local
                    && scope == self.names_global
                {
                    memo.cyclic_lets.insert(id);
                }
            }
            Query::Param(scope, index) => {
                memo.params.insert((scope, index), BoundVal::Error);
            }
            Query::Input(input) => {
                memo.inputs.insert(input, BoundVal::Error);
            }
            Query::State(state) => {
                memo.states.insert(state, BoundVal::Error);
            }
            Query::Attr(item, attr) => {
                memo.attrs.insert((item, attr), BoundVal::Error);
            }
            Query::Expr(..) | Query::Destination(_) | Query::Sig(_) | Query::Eval(..) => {}
        }
    }
}

type Chosen = (
    Code,
    Loc,
    Vec<(Loc, String)>,
    (String, Option<String>),
    usize,
);

#[derive(PartialEq, Eq, Hash)]
struct Merged {
    code: Code,
    loc: Loc,
    before: Vec<(Loc, String)>,
    message: String,
    outermost: Option<Loc>,
}
