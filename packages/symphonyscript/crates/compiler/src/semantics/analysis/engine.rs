use std::collections::{BTreeMap, VecDeque};

use symphonyscript_core::Vocabulary;

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
use crate::semantics::collection::{Collected, Variants};
use crate::semantics::ir::{Hir, Loc, PlacementId};
use crate::semantics::reporting::Report;

pub(crate) struct Engine<'a> {
    pub hir: &'a Hir,
    pub vocabulary: &'a Vocabulary,
    pub collected: &'a Collected,
    pub variants: &'a Variants,
    pub report: Report,
    pub scopes: Vec<Scope>,
    pub nodes: Vec<Node>,
    pub arrays: FastSet<NodeId>,
    pub bindings: Vec<Binding>,
    pub instances: Vec<Instance>,
    pub wires: Vec<Wire>,
    pub uses: Vec<PatternUse>,
    pub top_instances: BTreeMap<PlacementId, InstanceId>,
    pub top_level: Vec<InstanceId>,
    pub source_global: ScopeId,
    pub prelude_global: ScopeId,
    pub memo: Memo,
    pub prebuilt: FastMap<(Site, ScopeId), ItemRef>,
    pub counters: FastMap<(PathParent, &'a str), u32>,
    pub body_queue: VecDeque<InstanceId>,
    pub expanded: usize,
    pub steps: usize,
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
    pub machines: FastMap<super::call_key::CallKey, super::evaluation::Machine>,
    pub frame_need: Vec<(NodeId, super::eval_ty::EvalTy)>,
    pub frame_pushed: bool,
    pub call_count: usize,
    pub inlined: u64,
    pub inline_limit_reported: bool,
    pub call_limit_reported: bool,
    pub typing: super::infer::Typing,
    pub deferred: Vec<super::deferred::Deferred>,
    waiting: FastSet<Query>,
    cyclic: FastSet<(Query, Query)>,
    current: Option<Query>,
    missing_scope: Scope,
    missing_node: Node,
    missing_binding: Binding,
}

impl<'a> Engine<'a> {
    pub fn new(
        hir: &'a Hir,
        vocabulary: &'a Vocabulary,
        collected: &'a Collected,
        variants: &'a Variants,
    ) -> Self {
        let mut engine = Self {
            hir,
            vocabulary,
            collected,
            variants,
            report: Report::default(),
            scopes: Vec::new(),
            nodes: Vec::new(),
            arrays: FastSet::default(),
            bindings: Vec::new(),
            instances: Vec::new(),
            wires: Vec::new(),
            uses: Vec::new(),
            top_instances: BTreeMap::new(),
            top_level: Vec::new(),
            source_global: ScopeId(0),
            prelude_global: ScopeId(1),
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
            call_count: 0,
            inlined: 0,
            inline_limit_reported: false,
            call_limit_reported: false,
            typing: super::infer::Typing::default(),
            deferred: Vec::new(),
            waiting: FastSet::default(),
            cyclic: FastSet::default(),
            current: None,
            missing_scope: Scope {
                kind: ScopeKind::Global { prelude: false },
                parent: None,
                chain: Chain::default(),
                template: None,
            },
            missing_node: Node {
                op: Op::Bool(false),
                loc: Loc::default(),
                scope: ScopeId(0),
            },
            missing_binding: Binding {
                kind: super::binding_kind::BindingKind::Let,
                init: NodeId(u32::MAX),
                annotation: None,
                name: String::new(),
                loc: Loc::default(),
            },
        };
        engine.source_global =
            engine.scope(ScopeKind::Global { prelude: false }, None, Chain::default());
        engine.prelude_global =
            engine.scope(ScopeKind::Global { prelude: true }, None, Chain::default());
        engine
    }

    pub fn demand(&mut self, query: Query) {
        let mut stack: Vec<(Query, Option<usize>)> = vec![(query, None)];
        let mut stalled = 0;
        while let Some((top, _)) = stack.last().copied() {
            let position = stack.len() - 1;
            if self.is_done(top) {
                stack.pop();
                self.waiting.remove(&top);
                stalled = 0;
                continue;
            }
            let mut need = Vec::new();
            self.current = Some(top);
            let done = self.run(top, &mut need);
            self.current = None;
            if done {
                stack.pop();
                self.waiting.remove(&top);
                stalled = 0;
                continue;
            }
            self.waiting.insert(top);
            let mut progressed = false;
            for dep in need {
                if self.waiting.contains(&dep) {
                    if self.cyclic.insert((top, dep)) {
                        self.cycle(&stack, position, dep);
                        progressed = true;
                    }
                } else if !self.is_done(dep) {
                    stack.push((dep, Some(position)));
                    progressed = true;
                }
            }
            if progressed {
                stalled = 0;
            } else {
                stalled += 1;
                if stalled > 2 {
                    stack.pop();
                    self.waiting.remove(&top);
                    stalled = 0;
                }
            }
        }
    }

    pub fn in_progress(&self, query: Query) -> bool {
        self.waiting.contains(&query)
    }

    fn is_done(&self, query: Query) -> bool {
        let memo = &self.memo;
        match query {
            Query::Expr(expr, scope) => memo.exprs.contains_key(&(expr, scope)),
            Query::Let(scope, local) => memo.lets.contains_key(&(scope, local)),
            Query::Param(scope, index) => memo.params.contains_key(&(scope, index)),
            Query::Input(input) => memo.inputs.contains_key(&input),
            Query::Attr(item, attr) => memo.attrs.contains_key(&(item, attr)),
            Query::Destination(wire) => memo.destinations.contains_key(&wire),
            Query::Sig(node) => memo.sigs.contains_key(&node),
            Query::Eval(node, ty) => memo.evals.contains_key(&(node, ty)),
        }
    }

    fn run(&mut self, query: Query, need: &mut Vec<Query>) -> bool {
        match query {
            Query::Expr(expr, scope) => self.run_expr(expr, scope, need),
            Query::Let(scope, local) => self.run_let(scope, local, need),
            Query::Param(scope, index) => self.run_param(scope, index, need),
            Query::Input(input) => self.run_input(input, need),
            Query::Attr(item, attr) => self.run_attr(item, attr, need),
            Query::Destination(wire) => self.run_destination(wire, need),
            Query::Sig(node) => self.run_sig(node, need),
            Query::Eval(node, ty) => self.run_eval(node, ty, need),
        }
    }

    fn poisoned(&self, dep: Query) -> bool {
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
        if self.poisoned(query) {
            return Some(BoundVal::Error);
        }
        let memo = &self.memo;
        let found = match query {
            Query::Let(scope, local) => memo.lets.get(&(scope, local)).cloned(),
            Query::Param(scope, index) => memo.params.get(&(scope, index)).cloned(),
            Query::Input(input) => memo.inputs.get(&input).cloned(),
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

    pub fn cval(&self, query: Query, need: &mut Vec<Query>) -> Option<CVal> {
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

    pub fn demand_val(&mut self, query: Query) -> Val {
        self.demand(query);
        let mut need = Vec::new();
        self.val(query, &mut need).unwrap_or(Val::Error)
    }

    pub fn demand_sig(&mut self, node: NodeId) -> Sig {
        self.demand(Query::Sig(node));
        self.memo.sigs.get(&node).copied().unwrap_or(Sig::ERROR)
    }

    pub fn demand_cval(&mut self, node: NodeId, ty: super::eval_ty::EvalTy) -> CVal {
        self.demand(Query::Eval(node, ty));
        self.memo
            .evals
            .get(&(node, ty))
            .cloned()
            .unwrap_or(CVal::Error)
    }

    pub fn scope(&mut self, kind: ScopeKind, parent: Option<ScopeId>, chain: Chain) -> ScopeId {
        let template = parent.and_then(|parent| self.scope_of(parent).template);
        self.scope_in(kind, parent, chain, template)
    }

    pub fn scope_in(
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

    pub fn template_of(&self, scope: ScopeId) -> Option<ScopeId> {
        self.scope_of(scope).template
    }

    pub fn scope_of(&self, id: ScopeId) -> &Scope {
        self.scopes.get(id.index()).unwrap_or(&self.missing_scope)
    }

    pub fn node(&self, id: NodeId) -> &Node {
        self.nodes.get(id.index()).unwrap_or(&self.missing_node)
    }

    pub fn binding(&self, id: BindingId) -> &Binding {
        self.bindings
            .get(id.index())
            .unwrap_or(&self.missing_binding)
    }

    pub fn push_node(&mut self, op: Op, loc: Loc, scope: ScopeId) -> NodeId {
        let id = NodeId(index(self.nodes.len()));
        if self.may_hold_array(&op) {
            self.arrays.insert(id);
        }
        self.nodes.push(Node { op, loc, scope });
        id
    }

    pub fn holds_array(&self, val: &Val) -> bool {
        matches!(val, Val::Scalar(node) if self.arrays.contains(node))
    }

    fn may_hold_array(&self, op: &Op) -> bool {
        match op {
            Op::Recursive { .. } | Op::Element { .. } | Op::Array(_) => true,
            Op::Use(operand) | Op::Annotate(_, operand) => self.arrays.contains(operand),
            Op::Read(binding) => self.arrays.contains(&self.binding(*binding).init),
            Op::If(_, then, otherwise) => {
                self.arrays.contains(then) || self.arrays.contains(otherwise)
            }
            Op::Match {
                arms, otherwise, ..
            } => {
                arms.iter().any(|(_, value)| self.arrays.contains(value))
                    || otherwise.is_some_and(|value| self.arrays.contains(&value))
            }
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Arg(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Pick { .. }
            | Op::Convert(..)
            | Op::Builtin(..)
            | Op::ToRead { .. } => false,
        }
    }

    pub fn push_binding(&mut self, binding: Binding) -> BindingId {
        let id = BindingId(index(self.bindings.len()));
        self.bindings.push(binding);
        id
    }

    pub fn chain(&self, scope: ScopeId) -> Chain {
        self.scope_of(scope).chain.clone()
    }

    pub fn error(&mut self, code: Code, loc: Loc, scope: ScopeId, message: impl Into<String>) {
        if self
            .template_of(scope)
            .is_some_and(|template| self.memo.probes.contains(&template))
        {
            return;
        }
        self.error_with_chain(code, loc, Vec::new(), scope, message);
    }

    pub fn error_with_chain(
        &mut self,
        code: Code,
        loc: Loc,
        before: Vec<Loc>,
        scope: ScopeId,
        message: impl Into<String>,
    ) {
        let chain = self.chain(scope);
        self.defer(code, loc, before, chain, message);
    }

    pub fn defer(
        &mut self,
        code: Code,
        loc: Loc,
        before: Vec<Loc>,
        chain: Chain,
        message: impl Into<String>,
    ) {
        self.deferred.push(super::deferred::Deferred {
            code,
            loc,
            before,
            chain,
            message: message.into(),
        });
    }

    pub fn flush_diagnostics(&mut self) {
        let mut reported: FastSet<Merged> = FastSet::default();
        for deferred in std::mem::take(&mut self.deferred) {
            for path in deferred.chain.outermost_paths() {
                let key = Merged {
                    code: deferred.code,
                    loc: deferred.loc,
                    before: deferred.before.clone(),
                    message: deferred.message.clone(),
                    outermost: path.last().copied(),
                };
                if !reported.insert(key) {
                    continue;
                }
                let mut secondary = deferred.before.clone();
                secondary.extend(path);
                self.report.error_with(
                    self.hir,
                    deferred.code,
                    deferred.loc,
                    &secondary,
                    deferred.message.clone(),
                );
            }
        }
    }

    pub fn error_at_node(&mut self, code: Code, node: NodeId, message: impl Into<String>) {
        let (loc, scope) = {
            let node = self.node(node);
            (node.loc, node.scope)
        };
        self.error(code, loc, scope, message);
    }

    pub fn read(&mut self, bound: &BoundVal, loc: Loc, scope: ScopeId) -> Val {
        match bound {
            BoundVal::Scalar(item) => Val::Scalar(self.read_one(*item, loc, scope)),
            BoundVal::Array(array) => {
                let (shape, items) = array.as_ref();
                let elements = items
                    .iter()
                    .map(|item| self.read_one(*item, loc, scope))
                    .collect();
                Val::array(shape.clone(), elements)
            }
            BoundVal::Error => Val::Error,
        }
    }

    fn read_one(&mut self, item: Bound, loc: Loc, scope: ScopeId) -> NodeId {
        match item {
            Bound::Cell(binding) => self.push_node(Op::Read(binding), loc, scope),
            Bound::Value(node) => self.push_node(Op::Use(node), loc, scope),
        }
    }

    fn cycle(&mut self, stack: &[(Query, Option<usize>)], top: usize, dep: Query) {
        let mut path = Vec::new();
        let mut current = Some(top);
        while let Some(position) = current {
            let Some(&(query, parent)) = stack.get(position) else {
                break;
            };
            path.push(query);
            if query == dep {
                break;
            }
            current = parent;
        }
        path.reverse();
        let local_lets = path.iter().all(|query| match query {
            Query::Let(_, LetRef::Local(_)) => true,
            other => self.declaration(*other).is_none(),
        });
        if local_lets {
            return;
        }
        let mut names = Vec::new();
        let mut locs = Vec::new();
        for query in path {
            if let Some((name, loc)) = self.declaration(query) {
                names.push(name);
                locs.push(loc);
            }
        }
        let Some(first) = names.first().cloned() else {
            return;
        };
        names.push(first);
        let Some(&loc) = locs.first() else {
            return;
        };
        let message = format!("these declarations reach themselves: {}", names.join(" → "));
        let secondary = locs.get(1..).unwrap_or_default();
        self.report
            .error_with(self.hir, Code::DeclarationCycle, loc, secondary, message);
    }
}

#[derive(PartialEq, Eq, Hash)]
struct Merged {
    code: Code,
    loc: Loc,
    before: Vec<Loc>,
    message: String,
    outermost: Option<Loc>,
}
