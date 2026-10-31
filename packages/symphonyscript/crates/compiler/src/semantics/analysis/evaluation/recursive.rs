use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use symphonyscript_core::FamilyId;

use super::checks::Checks;
use super::ctx::Ctx;
use super::frame::Frame;
use super::frame_spare::FrameSpare;
use super::frame_values::FrameValues;
use super::is_float_ty::is_float_ty;
use super::machine::Machine;
use super::machine_key::MachineKey;
use super::mode::Mode;
use super::own_type::own_type;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::arg_key::ArgKey;
use crate::semantics::analysis::arg_reference::ArgReference;
use crate::semantics::analysis::arg_shape::ArgShape;
use crate::semantics::analysis::arg_sig::ArgSig;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::call_done::CallDone;
use crate::semantics::analysis::call_parts::CallParts;
use crate::semantics::analysis::call_scratch::CallScratch;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::feed::Feed;
use crate::semantics::analysis::frame_read::FrameRead;
use crate::semantics::analysis::ids::{CallId, NodeId, ScopeId, TemplateId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::limit::Limit;
use crate::semantics::analysis::limit_warning::warned;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::reachable::Reachable;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::sink::Sink;
use crate::semantics::analysis::slot_owner::SlotOwner;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::template_parts::TemplateParts;
use crate::semantics::analysis::val::Val;
use crate::semantics::analysis::width::Width;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::{ExprId, FnId};
use crate::semantics::reporting::grouped;

enum Template {
    Root(NodeId),
    Failed,
    Reentered,
}

enum Reuse {
    Serve(CVal),
    Unreferenced(CallId),
    Evaluate,
}

enum Reached {
    Call(CallId, TemplateId),
    Value(Option<CVal>),
}

struct Call {
    id: CallId,
    template: TemplateId,
    function: FnId,
    values: Vec<CVal>,
    feeds: Vec<Feed>,
}

impl Engine<'_> {
    pub(super) fn recursive(
        &mut self,
        node: NodeId,
        (function, args): (FnId, Rc<[Option<Val>]>),
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let held = match ctx {
            Ctx::Frame(caller) => match self.frame(caller)?.held {
                Some((held, ty, id)) if held == node && ty == own => Some(id),
                Some(_) | None => None,
            },
            Ctx::Global => None,
        };
        let (id, reached) = match held {
            Some(id) => (id, None),
            None => match self.reached_call(node, function, &args, own, ctx, need)? {
                Reached::Call(id, template) => (id, Some(template)),
                Reached::Value(value) => return Ok(value),
            },
        };
        let mode = match ctx {
            Ctx::Global => Mode::Taken,
            Ctx::Frame(caller) => {
                let record = self.frame_mut(caller)?;
                match record.checks {
                    Checks::Unchecked | Checks::Done => record.callees.push(id),
                    Checks::Checking { .. } => {}
                }
                self.mode_of(caller)?
            }
        };
        if let Ctx::Frame(caller) = ctx
            && mode == Mode::Untaken
            && self.machine_mut()?.open_keys.contains(&id)
        {
            self.taint(caller, id)?;
            return Ok(Some(CVal::Error));
        }
        match self.call_reuse(id, mode)? {
            Reuse::Serve(value) => {
                if let Ctx::Frame(caller) = ctx {
                    let height = self.memo.calls.get(&id).map_or(1, |done| done.height);
                    let depth = self.untaken_depth(caller)?;
                    if mode == Mode::Untaken && depth + height > self.limits.nesting_depth {
                        if depth > 0 {
                            self.pass_depth(caller, node, function)?;
                        }
                        return Ok(Some(CVal::Error));
                    }
                    self.raise_height(caller, height)?;
                }
                let request = self.sink;
                self.link(&request, Reachable::Call(id));
                self.reach_key(id, node, ctx)?;
                return Ok(Some(value));
            }
            Reuse::Unreferenced(opened) => {
                if let Ctx::Frame(caller) = ctx {
                    self.taint(caller, opened)?;
                }
                return Ok(Some(CVal::Error));
            }
            Reuse::Evaluate => {}
        }
        let template = match reached {
            Some(template) => template,
            None => match self.reached_call(node, function, &args, own, ctx, need)? {
                Reached::Call(_, template) => template,
                Reached::Value(value) => return Ok(value),
            },
        };
        let feeds = match ctx {
            Ctx::Global => self.memo.call_feeds.get(&node).cloned().unwrap_or_default(),
            Ctx::Frame(frame) => self.frame(frame)?.feeds.clone(),
        };
        let call = Call {
            id,
            template,
            function,
            values: std::mem::take(&mut self.call_scratch.values),
            feeds,
        };
        match ctx {
            Ctx::Global => self.run_machine(call, node, own, need),
            Ctx::Frame(_) => self.push_frame(call, node, own, need),
        }
    }

    fn reached_call(
        &mut self,
        node: NodeId,
        function: FnId,
        args: &[Option<Val>],
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Reached, Internal> {
        let mut scratch = std::mem::take(&mut self.call_scratch);
        scratch.clear();
        let reached = self.call_parts(node, args, own, ctx, need, &mut scratch);
        let reached = match reached {
            Ok(Some(())) if scratch.keys.contains(&ArgKey::Error) => {
                Ok(Reached::Value(Some(CVal::Error)))
            }
            Ok(Some(())) => {
                let feeds = match ctx {
                    Ctx::Global => self.memo.call_feeds.get(&node),
                    Ctx::Frame(frame) => Some(&self.frame(frame)?.feeds),
                };
                unit_set_into(feeds.map_or(&[], Vec::as_slice), &mut scratch.units);
                let template = TemplateId(self.memo.template_keys.intern(TemplateParts {
                    function,
                    args: &scratch.shapes,
                    units: &scratch.units,
                }));
                let id = CallId(self.memo.call_keys.intern(CallParts {
                    template,
                    args: &scratch.keys,
                    expected: own,
                }));
                Ok(Reached::Call(id, template))
            }
            Ok(None) => Ok(Reached::Value(None)),
            Err(error) => Err(error),
        };
        self.call_scratch = scratch;
        reached
    }

    fn call_parts(
        &mut self,
        node: NodeId,
        args: &[Option<Val>],
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
        scratch: &mut CallScratch,
    ) -> Result<Option<()>, Internal> {
        let mut missing = false;
        for arg in args {
            match arg {
                None => {
                    scratch.keys.push(ArgKey::Default);
                    scratch.shapes.push(ArgShape::Default);
                }
                Some(Val::Scalar(arg)) => {
                    let start = scratch.leaves.len();
                    let parts = (&mut scratch.leaves, &mut scratch.values);
                    match self.argument(*arg, own, ctx, need, parts)? {
                        Some(lengths) if lengths.is_empty() => match scratch.leaves.pop() {
                            Some((key, sig)) => {
                                scratch.keys.push(key);
                                scratch.shapes.push(ArgShape::Scalar(sig));
                            }
                            None => return Err(Internal::missed("argument")),
                        },
                        Some(lengths) => {
                            let (keys, sigs) = scratch.leaves.drain(start..).unzip();
                            scratch.keys.push(ArgKey::Array(keys));
                            scratch.shapes.push(ArgShape::Array(lengths, sigs));
                        }
                        None => missing = true,
                    }
                }
                Some(Val::Array(array)) => {
                    let start = scratch.leaves.len();
                    let first_value = scratch.values.len();
                    let mut inner: Option<Vec<u32>> = None;
                    let mut ragged = false;
                    let mut held = true;
                    for element in &array.elements {
                        let parts = (&mut scratch.leaves, &mut scratch.values);
                        match self.argument(*element, own, ctx, need, parts)? {
                            Some(lengths) => match &inner {
                                Some(first) => ragged |= *first != lengths,
                                None => inner = Some(lengths),
                            },
                            None => {
                                missing = true;
                                held = false;
                            }
                        }
                    }
                    if !held {
                        continue;
                    }
                    if ragged {
                        scratch.leaves.truncate(start);
                        scratch.values.truncate(first_value);
                        self.error_at_node(
                            Code::RaggedArray,
                            node,
                            "the elements of an array are arrays of one length, or all single values",
                        )?;
                        scratch.keys.push(ArgKey::Error);
                        continue;
                    }
                    let mut lengths = inner.unwrap_or_default();
                    lengths.extend(array.shape.lengths());
                    let (keys, sigs) = scratch.leaves.drain(start..).unzip();
                    scratch.keys.push(ArgKey::Array(keys));
                    scratch.shapes.push(ArgShape::Array(lengths, sigs));
                }
                Some(Val::Error) => scratch.keys.push(ArgKey::Error),
            }
        }
        Ok((!missing).then_some(()))
    }

    fn call_reuse(&self, id: CallId, mode: Mode) -> Result<Reuse, Internal> {
        let Some(done) = self.memo.calls.get(&id) else {
            return Ok(Reuse::Evaluate);
        };
        let open = |key: &CallId| {
            self.active
                .as_ref()
                .is_some_and(|machine| machine.open_keys.contains(key))
        };
        let still_open = done.opened.iter().find(|key| open(key)).copied();
        Ok(match (done.mode, mode) {
            (Mode::Taken, Mode::Taken) => {
                if still_open.is_none() && !done.opened.is_empty() {
                    Reuse::Evaluate
                } else {
                    Reuse::Serve(done.value.clone())
                }
            }
            (Mode::Taken, Mode::Untaken) => match self.through_open(done)? {
                Some(open) => Reuse::Unreferenced(open),
                None => Reuse::Serve(done.value.clone()),
            },
            (Mode::Untaken, Mode::Taken) => Reuse::Evaluate,
            (Mode::Untaken, Mode::Untaken) => {
                if let Some(open) = still_open {
                    Reuse::Unreferenced(open)
                } else if !done.opened.is_empty() {
                    Reuse::Evaluate
                } else {
                    match self.through_open(done)? {
                        Some(open) => Reuse::Unreferenced(open),
                        None => Reuse::Serve(done.value.clone()),
                    }
                }
            }
        })
    }

    fn through_open(&self, done: &CallDone) -> Result<Option<CallId>, Internal> {
        let Some(machine) = self.active.as_ref() else {
            return Ok(None);
        };
        if machine.rerun.is_empty() {
            return Ok(None);
        }
        let mut seen: FastSet<CallId> = FastSet::default();
        let mut pending: Vec<CallId> = done.callees.to_vec();
        while let Some(key) = pending.pop() {
            CountedWalk::RecursiveThroughOpen.step();
            if !seen.insert(key) {
                continue;
            }
            if machine.open_keys.contains(&key) {
                return Ok(Some(key));
            }
            if let Some(callee) = self.memo.calls.get(&key) {
                pending.extend(callee.callees.iter().copied());
            }
        }
        Ok(None)
    }

    fn taint(&mut self, frame: usize, opened: CallId) -> Result<(), Internal> {
        self.frame_mut(frame)?.opened.insert(opened);
        Ok(())
    }

    fn untaken_depth(&self, frame: usize) -> Result<u64, Internal> {
        let record = self.frame(frame)?;
        Ok(match record.mode {
            Mode::Taken => 0,
            Mode::Untaken => record.untaken_depth,
        })
    }

    fn raise_height(&mut self, frame: usize, callee: u64) -> Result<(), Internal> {
        let cap = self.limits.nesting_depth.saturating_add(1);
        let record = self.frame_mut(frame)?;
        match record.checks {
            Checks::Unchecked | Checks::Done => {
                record.height = record.height.max(callee.saturating_add(1)).min(cap);
            }
            Checks::Checking { .. } => {}
        }
        Ok(())
    }

    fn pass_depth(&mut self, caller: usize, node: NodeId, last: FnId) -> Result<(), Internal> {
        let start = self.frame(caller)?.untaken_start;
        let mut names: Vec<String> = Vec::new();
        let functions: Vec<FnId> = self
            .active
            .as_ref()
            .ok_or(Internal::missed("evaluation machine"))?
            .frames
            .get(start..=caller)
            .unwrap_or_default()
            .iter()
            .filter_map(|frame| frame.id)
            .filter_map(|id| self.call_function(id))
            .collect();
        for function in functions.into_iter().chain([last]) {
            let name = self.function_name(function)?;
            if !names.contains(&name) {
                names.push(name);
            }
        }
        self.error_at_node(
            Code::RecursionDepth,
            node,
            format!(
                "the evaluation goes deeper than {} nested calls: {}",
                grouped(self.limits.nesting_depth),
                names.join(" → ")
            ),
        )?;
        self.machine_mut()?.passed = Some(start);
        Ok(())
    }

    fn drop_passed(&mut self, start: usize) -> Result<(), Internal> {
        let cap = self.limits.nesting_depth.saturating_add(1);
        let machine = self.machine_mut()?;
        let dropped = machine.frames.split_off(start);
        machine.work.retain(|(frame, ..)| *frame < start);
        for frame in &dropped {
            if let Some(id) = frame.id {
                machine.open_keys.remove(&id);
                machine.rerun.remove(&id);
            }
        }
        let Some(id) = dropped.first().and_then(|frame| frame.id) else {
            return Err(Internal::missed("untaken start"));
        };
        let kept = self
            .memo
            .calls
            .get(&id)
            .is_some_and(|old| old.mode == Mode::Taken);
        if !kept {
            let done = CallDone {
                value: CVal::Error,
                mode: Mode::Untaken,
                opened: Vec::new(),
                height: cap,
                callees: Vec::new(),
            };
            self.memo.calls.insert(id, done);
        }
        Ok(())
    }

    fn call_function(&self, id: CallId) -> Option<FnId> {
        let template = self.memo.call_keys.key(id.0)?.template;
        Some(self.memo.template_keys.key(template.0)?.function)
    }

    fn template_scope(&self, template: TemplateId) -> Option<ScopeId> {
        let position = usize::try_from(template.0).ok()?;
        self.memo.template_scopes.get(position).copied().flatten()
    }

    fn frame_mut(&mut self, frame: usize) -> Result<&mut Frame, Internal> {
        self.machine_mut()?
            .frames
            .get_mut(frame)
            .ok_or(Internal::missed("evaluation frame"))
    }

    pub(super) fn mode_of(&self, frame: usize) -> Result<Mode, Internal> {
        let record = self.frame(frame)?;
        Ok(match (record.mode, &record.checks) {
            (Mode::Untaken, _) | (Mode::Taken, Checks::Checking { .. }) => Mode::Untaken,
            (Mode::Taken, Checks::Unchecked | Checks::Done) => Mode::Taken,
        })
    }

    pub(super) fn frame_sink(&self, frame: usize) -> Result<Sink, Internal> {
        Ok(match self.mode_of(frame)? {
            Mode::Taken => self.frame(frame)?.sink,
            Mode::Untaken => Sink::Discard,
        })
    }

    fn argument(
        &mut self,
        arg: NodeId,
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
        (leaves, values): (&mut Vec<(ArgKey, ArgSig)>, &mut Vec<CVal>),
    ) -> Result<Option<Vec<u32>>, Internal> {
        let sig = some!(self.sig(arg, need));
        let settled = match sig.kind {
            Kind::Bare => self.carried_enum_of(arg)?,
            Kind::Int
            | Kind::Float
            | Kind::Bool
            | Kind::Str
            | Kind::Node
            | Kind::Enum(_)
            | Kind::Any
            | Kind::Error => None,
        };
        if let Some(id) = settled {
            let value = some!(self.child(arg, EvalTy::Enum(id), ctx, need)?);
            leaves.push(arg_key(&value, false, EvalTy::Enum(id), None));
            values.push(value);
            return Ok(Some(Vec::new()));
        }
        if sig.kind == Kind::Bare {
            self.unplaced_variant(arg)?;
            leaves.push(arg_key(&CVal::Error, false, own, None));
            values.push(CVal::Error);
            return Ok(Some(Vec::new()));
        }
        let flexible = sig.width == Width::Flexible;
        let ty = if flexible && sig.kind == Kind::Int {
            EvalTy::Exact
        } else {
            own_type(&sig, own)
        };
        let value = some!(self.typed_child(arg, (ty, false), ctx, need)?);
        let CVal::Array(array) = value else {
            leaves.push(arg_key(&value, flexible, ty, sig.family));
            values.push(value);
            return Ok(Some(Vec::new()));
        };
        for leaf in array.leaves() {
            leaves.push(arg_key(&leaf, flexible, ty, sig.family));
            values.push(leaf);
        }
        Ok(Some(array.lengths()))
    }

    pub(in crate::semantics::analysis) fn instantiate(
        &mut self,
        function: FnId,
        shapes: &[ArgShape],
        call: NodeId,
    ) -> Result<ScopeId, Internal> {
        let (loc, caller) = {
            let record = self.node(call)?;
            (record.loc, record.scope)
        };
        let mut slot = 0u32;
        let mut params = Vec::new();
        let mut arguments = Vec::new();
        for shape in shapes {
            let source = match shape {
                ArgShape::Default => ParamSource::Default,
                ArgShape::Scalar(sig) => {
                    let node = self.arg_node(&mut slot, sig, loc, caller)?;
                    arguments.push(node);
                    ParamSource::Bound(BoundVal::Scalar(Bound::Value(node)))
                }
                ArgShape::Array(lengths, sigs) => {
                    let mut bounds = Vec::with_capacity(sigs.len());
                    for sig in sigs {
                        let node = self.arg_node(&mut slot, sig, loc, caller)?;
                        arguments.push(node);
                        bounds.push(Bound::Value(node));
                    }
                    let shape = crate::semantics::analysis::shape::Shape::from_lengths(lengths);
                    ParamSource::Bound(BoundVal::Array(Rc::new((shape, bounds))))
                }
            };
            params.push(source);
        }
        let defaulted: Vec<usize> = shapes
            .iter()
            .enumerate()
            .filter(|(_, shape)| matches!(shape, ArgShape::Default))
            .map(|(position, _)| position)
            .collect();
        self.charge_evaluated(function, &defaulted, loc, caller)?;
        let template = self.call_scope(function, params, loc, caller, true)?;
        for node in arguments {
            let count = self
                .memo
                .slot_counts
                .entry(SlotOwner::Template(template))
                .or_insert(0);
            let order = Some((template, *count));
            *count = count.saturating_add(1);
            self.nodes
                .get_mut(node.index())
                .ok_or(Internal::missed("node"))?
                .order = order;
        }
        Ok(template)
    }

    fn template(
        &mut self,
        call: &Call,
        node: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Template>, Internal> {
        let function = call.function;
        let scope = match self.template_scope(call.template) {
            Some(scope) => scope,
            None => {
                let shapes = self
                    .memo
                    .template_keys
                    .key(call.template.0)
                    .map(|key| key.args.clone())
                    .ok_or(Internal::missed("template"))?;
                let scope = self.instantiate(function, &shapes, node)?;
                self.memo.template_feeds.insert(scope, call.feeds.clone());
                let position = usize::try_from(call.template.0).unwrap_or(usize::MAX);
                if self.memo.template_scopes.len() <= position {
                    self.memo
                        .template_scopes
                        .resize(position.saturating_add(1), None);
                }
                if let Some(slot) = self.memo.template_scopes.get_mut(position) {
                    *slot = Some(scope);
                }
                scope
            }
        };
        let body = self.hir.fn_decl(function)?.value;
        let val = some!(self.val(Query::Expr(body, scope), need));
        let Some(root) = self.boxed_body(body, scope, &val)? else {
            return Ok(Some(Template::Failed));
        };
        if self.in_progress(Query::Sig(root)) {
            return Ok(Some(Template::Reentered));
        }
        let sig = some!(self.sig(root, need));
        if sig.reactive && self.hir.fn_decl(function)?.faulty_head {
            return Ok(Some(Template::Failed));
        }
        if sig.reactive {
            self.reactive_body(node, function)?;
            return Ok(Some(Template::Failed));
        }
        self.template_units(scope, root)?;
        Ok(Some(Template::Root(root)))
    }

    pub(super) fn frame_read(
        &mut self,
        read: &FrameRead,
        frame: usize,
        (own, flows): (EvalTy, bool),
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let ctx = Ctx::Frame(frame);
        let item = match some!(self.child(read.start, EvalTy::Node, ctx, need)?) {
            CVal::Node(item) => item,
            CVal::Int(_)
            | CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Array(_)
            | CVal::Error => return Ok(Some(CVal::Error)),
        };
        let chosen = read
            .arms
            .iter()
            .find(|(candidate, _)| *candidate == item)
            .map(|(_, arm)| *arm);
        let open = read.open.iter().find(|(candidate, _)| *candidate == item);
        let arm = match (chosen, open) {
            (Some(arm), _) => arm,
            (None, Some((_, path))) => {
                if self.live_candidate_cycle(item, &read.name.text, path)? {
                    return Ok(Some(CVal::Error));
                }
                match some!(self.open_arm(read, item, need)?) {
                    Some(arm) => arm,
                    None => return Ok(Some(CVal::Error)),
                }
            }
            (None, None) => {
                if !self.holds_name(item, &read.name.text)? {
                    self.lacking_choice(read, item)?;
                    return Ok(Some(CVal::Error));
                }
                match some!(self.open_arm(read, item, need)?) {
                    Some(arm) => arm,
                    None => return Ok(Some(CVal::Error)),
                }
            }
        };
        if some!(self.sig(arm, need)).reactive {
            if self.mode_of(frame)? == Mode::Untaken {
                return Ok(Some(CVal::Error));
            }
            let fault = self.fault_count;
            self.fault_count += 1;
            self.frame_mut(frame)?.faults.insert(fault);
            self.memo.reactive_faults.insert(fault);
            return Ok(Some(CVal::Error));
        }
        self.typed_child(arm, (own, flows), ctx, need)
    }

    fn holds_name(&mut self, item: ItemRef, name: &str) -> Result<bool, Internal> {
        Ok(self.child_named(item, name)?.is_some() || self.attr_named(item, name)?.is_some())
    }

    fn lacking_choice(&mut self, read: &FrameRead, item: ItemRef) -> Result<(), Internal> {
        if self.untaken()?
            || self.holds_name(item, &read.name.text)?
            || self.may_name(item, &read.name.text)?
        {
            return Ok(());
        }
        let owner = self.item_description(item)?;
        self.error(
            Code::UnknownAttribute,
            read.name.loc,
            read.scope,
            format!(
                "{owner} has no child, wire or attribute `{}`",
                read.name.text
            ),
        )
    }

    fn frame_template(&self, frame: usize) -> Result<Option<ScopeId>, Internal> {
        let root = self.frame(frame)?.root;
        self.template_of(self.node(root)?.scope)
    }

    fn template_heads(&self, template: Option<ScopeId>) -> usize {
        template
            .and_then(|template| self.memo.template_heads.get(&template))
            .map_or(0, Vec::len)
    }

    fn template_head(&self, template: Option<ScopeId>, position: usize) -> Option<NodeId> {
        CountedWalk::RecursiveFrameHeads.step();
        self.memo
            .template_heads
            .get(&template?)?
            .get(position)
            .copied()
    }

    fn begin_checks(&mut self, frame: usize, root: NodeId, ty: EvalTy) -> Result<bool, Internal> {
        let record = self.frame(frame)?;
        if record.mode == Mode::Untaken || !matches!(record.checks, Checks::Unchecked) {
            return Ok(false);
        }
        let template = self.frame_template(frame)?;
        let mut pending = Vec::new();
        for position in 0..self.template_heads(template) {
            let head = self
                .template_head(template, position)
                .ok_or(Internal::missed("frame head"))?;
            CountedWalk::RecursiveBeginChecks.step();
            let read = self
                .memo
                .frame_reads
                .get(&head)
                .ok_or(Internal::missed("frame read"))?;
            pending.push(read.start);
        }
        for position in 0..self.template_calls(template) {
            let call = self
                .template_call(template, position)
                .ok_or(Internal::missed("frame call"))?;
            self.reference_args(call, &mut pending)?;
        }
        let Some(start) = pending.pop() else {
            return Ok(false);
        };
        let machine = self.machine_mut()?;
        machine
            .frames
            .get_mut(frame)
            .ok_or(Internal::missed("evaluation frame"))?
            .checks = Checks::Checking { start, pending };
        machine.work.push((frame, root, ty));
        machine.work.push((frame, start, EvalTy::Node));
        Ok(true)
    }

    fn next_start(&mut self, frame: usize, node: NodeId, ty: EvalTy) -> Result<(), Internal> {
        let machine = self.machine_mut()?;
        let record = machine
            .frames
            .get_mut(frame)
            .ok_or(Internal::missed("evaluation frame"))?;
        let Checks::Checking { start, pending } = &mut record.checks else {
            return Ok(());
        };
        if *start != node || ty != EvalTy::Node {
            return Ok(());
        }
        if let Some(next) = pending.pop() {
            *start = next;
            machine.work.push((frame, next, EvalTy::Node));
        }
        Ok(())
    }

    fn ends_checks(&self, frame: usize, node: NodeId, ty: EvalTy) -> Result<bool, Internal> {
        let record = self.frame(frame)?;
        Ok(matches!(record.checks, Checks::Checking { .. })
            && record.root == node
            && record.ty == ty)
    }

    fn check_heads(&mut self, frame: usize, need: &mut Vec<Query>) -> Result<bool, Internal> {
        let enclosing = self.fault_frame.replace(frame);
        let called = self.checked_calls(frame, need);
        self.fault_frame = enclosing;
        if !called? {
            return Ok(false);
        }
        self.machine_mut()?
            .frames
            .get_mut(frame)
            .ok_or(Internal::missed("evaluation frame"))?
            .checks = Checks::Done;
        let enclosing = self.fault_frame.replace(frame);
        let checked = self.checked_heads(frame);
        self.fault_frame = enclosing;
        checked.map(|()| true)
    }

    fn template_calls(&self, template: Option<ScopeId>) -> usize {
        template
            .and_then(|template| self.memo.template_calls.get(&template))
            .map_or(0, Vec::len)
    }

    fn template_call(&self, template: Option<ScopeId>, position: usize) -> Option<NodeId> {
        CountedWalk::RecursiveCallHeads.step();
        self.memo
            .template_calls
            .get(&template?)?
            .get(position)
            .copied()
    }

    fn reference_args(&self, call: NodeId, references: &mut Vec<NodeId>) -> Result<(), Internal> {
        let Op::Recursive { args, .. } = &self.node(call)?.op else {
            return Ok(());
        };
        for arg in args.iter().flatten() {
            if let Val::Scalar(node) = arg
                && self.chosen_reference(*node)?
            {
                references.push(*node);
            }
        }
        Ok(())
    }

    fn chosen_reference(&self, node: NodeId) -> Result<bool, Internal> {
        let Some(sig) = self.memo.sigs.get(&node) else {
            return Ok(false);
        };
        Ok(sig.kind == Kind::Node && sig.frame && !sig.reactive && !sig.is_error())
    }

    fn checked_calls(&mut self, frame: usize, need: &mut Vec<Query>) -> Result<bool, Internal> {
        let mut complete = true;
        let template = self.frame_template(frame)?;
        for position in 0..self.template_calls(template) {
            let call = self
                .template_call(template, position)
                .ok_or(Internal::missed("frame call"))?;
            let Op::Recursive { function, args } = self.node(call)?.op.clone() else {
                continue;
            };
            let mut references = Vec::new();
            self.reference_args(call, &mut references)?;
            if references.is_empty() {
                continue;
            }
            let mut shapes = Vec::with_capacity(args.len());
            let mut chosen = Vec::new();
            let mut unchosen = false;
            let mut missing = false;
            for arg in args.iter() {
                if let Some(Val::Scalar(node)) = arg
                    && self.chosen_reference(*node)?
                {
                    match self.frame_value(frame, *node, EvalTy::Node)? {
                        Some(CVal::Node(item)) => {
                            chosen.push(item);
                            shapes.push(ArgShape::Scalar(ArgSig {
                                kind: Kind::Node,
                                width: Width::Flexible,
                                family: self.known_sig(*node)?.family,
                                node: ArgReference::Instance(item),
                            }));
                        }
                        Some(
                            CVal::Int(_)
                            | CVal::F32(_)
                            | CVal::F64(_)
                            | CVal::Bool(_)
                            | CVal::Enum(..)
                            | CVal::Str(_)
                            | CVal::Array(_)
                            | CVal::Error,
                        )
                        | None => unchosen = true,
                    }
                    continue;
                }
                match self.family_shape(arg.as_ref(), need)? {
                    Some(shape) => shapes.push(shape),
                    None => missing = true,
                }
            }
            if chosen.is_empty() || unchosen {
                continue;
            }
            if missing {
                complete = false;
                continue;
            }
            let Some(settled) = self.chosen_family(call, function, &shapes, need)? else {
                complete = false;
                continue;
            };
            let Some(settled) = settled else {
                continue;
            };
            let body = Query::Expr(self.hir.fn_decl(function)?.value, settled);
            let reads: Vec<NodeId> = self
                .memo
                .template_steps
                .get(&settled)
                .cloned()
                .unwrap_or_default();
            for read in reads {
                let Some(&(item, attr)) = self.memo.attr_reads.get(&read) else {
                    continue;
                };
                if !chosen.contains(&item) || self.machine_mut()?.walked.contains(&read) {
                    continue;
                }
                match self.evaluating_path(read)? {
                    Some(path) => {
                        let mut steps = vec![Query::Attr(item, attr)];
                        steps.extend(
                            path.iter()
                                .filter_map(|node| self.memo.value_bindings.get(node).copied()),
                        );
                        steps.push(body);
                        self.draw_evaluation_cycle((item, attr), steps)?;
                    }
                    None => {
                        self.machine_mut()?.walked.insert(read);
                    }
                }
            }
        }
        Ok(complete)
    }

    fn checked_heads(&mut self, frame: usize) -> Result<(), Internal> {
        let template = self.frame_template(frame)?;
        for position in 0..self.template_heads(template) {
            let head = self
                .template_head(template, position)
                .ok_or(Internal::missed("frame head"))?;
            CountedWalk::RecursiveCheckedHeads.step();
            let read = self
                .memo
                .frame_reads
                .get(&head)
                .cloned()
                .ok_or(Internal::missed("frame read"))?;
            let item = match self.frame_value(frame, read.start, EvalTy::Node)? {
                Some(CVal::Node(item)) => item,
                Some(
                    CVal::Int(_)
                    | CVal::F32(_)
                    | CVal::F64(_)
                    | CVal::Bool(_)
                    | CVal::Enum(..)
                    | CVal::Str(_)
                    | CVal::Array(_)
                    | CVal::Error,
                )
                | None => continue,
            };
            if read.arms.iter().any(|(candidate, _)| *candidate == item) {
                continue;
            }
            match read.open.iter().find(|(candidate, _)| *candidate == item) {
                Some((_, path)) => {
                    self.live_candidate_cycle(item, &read.name.text, path)?;
                }
                None => self.lacking_choice(&read, item)?,
            }
        }
        Ok(())
    }

    pub(super) fn untaken_unevaluated(
        &self,
        frame: usize,
        node: NodeId,
        ty: EvalTy,
    ) -> Result<bool, Internal> {
        Ok(self.mode_of(frame)? == Mode::Untaken
            && !self.memo.evals.contains_key(&(node, ty))
            && !self.poisoned(Query::Eval(node, ty)))
    }

    pub(in crate::semantics::analysis) fn untaken(&self) -> Result<bool, Internal> {
        match (self.fault_frame, self.active.as_ref()) {
            (Some(frame), Some(_)) => Ok(self.mode_of(frame)? == Mode::Untaken),
            (Some(_), None) | (None, _) => Ok(false),
        }
    }

    pub(super) fn reactive_body(&mut self, call: NodeId, function: FnId) -> Result<(), Internal> {
        let reading = format!(
            "the body of the recursive function `{}` reads a cell",
            self.function_name(function)?
        );
        self.reactive_recursion(call, function, (reading, true))
    }

    fn result_fits(
        &mut self,
        root: NodeId,
        own: EvalTy,
        call: NodeId,
        function: FnId,
    ) -> Result<bool, Internal> {
        let kind = self.known_sig(root)?.kind;
        let not_numeric = matches!(
            own,
            EvalTy::Bool | EvalTy::Str | EvalTy::Enum(_) | EvalTy::Node
        );
        let fits = match kind {
            Kind::Float => {
                !not_numeric
                    && !matches!(own, EvalTy::Exact | EvalTy::U8 | EvalTy::I32 | EvalTy::I64)
            }
            Kind::Int => !not_numeric,
            Kind::Bool => matches!(own, EvalTy::Bool | EvalTy::Default),
            Kind::Str => matches!(own, EvalTy::Str | EvalTy::Default),
            Kind::Enum(id) => own == EvalTy::Enum(id) || own == EvalTy::Default,
            Kind::Node | Kind::Bare | Kind::Any | Kind::Error => true,
        };
        if !fits {
            self.error_at_node(
                Code::TypeMismatch,
                call,
                format!(
                    "this call of `{}` yields a value of the wrong type here",
                    self.function_name(function)?
                ),
            )?;
        }
        Ok(fits)
    }

    fn arg_node(
        &mut self,
        slot: &mut u32,
        sig: &ArgSig,
        loc: crate::semantics::ir::Loc,
        scope: ScopeId,
    ) -> Result<NodeId, Internal> {
        if let ArgReference::Instance(item) = sig.node {
            *slot += 1;
            return self.push_node(Op::NodeRef(item), loc, scope);
        }
        let node = self.push_node(Op::Arg(*slot), loc, scope)?;
        *slot += 1;
        let mut arg_sig = Sig::of(sig.kind, sig.width, false, sig.family);
        arg_sig.frame = true;
        self.memo.arg_sigs.insert(node, arg_sig);
        if let ArgReference::Among(instances) = &sig.node {
            self.memo.arg_instances.insert(node, instances.clone());
        }
        Ok(node)
    }

    fn run_machine(
        &mut self,
        call: Call,
        node: NodeId,
        own: EvalTy,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let root_key = MachineKey::Call(call.id);
        let machine = match self.machines.remove(&root_key) {
            Some(machine) => machine,
            None => {
                let root = match some!(self.template(&call, node, need)?) {
                    Template::Root(root) if self.result_fits(root, own, node, call.function)? => {
                        root
                    }
                    Template::Reentered => return Ok(Some(CVal::Error)),
                    Template::Root(_) | Template::Failed => {
                        return self.failed_call(&call, node, Ctx::Global);
                    }
                };
                let defaults = some!(self.default_work(&call, need)?);
                if !self.within_call_limit(node, call.function, call.id)? {
                    return Ok(Some(CVal::Error));
                }
                let ty = own;
                let template = self.template_of(self.node(root)?.scope)?;
                let templates = template.into_iter().collect();
                let owner = template.map_or(SlotOwner::Root(root), SlotOwner::Template);
                let mut work = vec![(0, root, ty)];
                work.extend(defaults.into_iter().map(|(node, ty)| (0, node, ty)));
                let rerun: FastSet<CallId> = self
                    .memo
                    .calls
                    .contains_key(&call.id)
                    .then_some(call.id)
                    .into_iter()
                    .collect();
                Machine {
                    frames: vec![Frame {
                        run: None,
                        args: call.values,
                        call: node,
                        root,
                        ty,
                        expected: own,
                        feeds: call.feeds,
                        owner,
                        values: FrameValues::default(),
                        loops: FrameValues::default(),
                        held: None,
                        depth: 1,
                        faults: FastSet::default(),
                        templates,
                        checks: Checks::Unchecked,
                        mode: Mode::Taken,
                        sink: Sink::Into(Reachable::Call(call.id)),
                        id: Some(call.id),
                        opened: FastSet::default(),
                        untaken_depth: 0,
                        untaken_start: 0,
                        height: 1,
                        callees: Vec::new(),
                    }],
                    work,
                    open_keys: [call.id].into_iter().collect(),
                    rerun,
                    too_deep: false,
                    passed: None,
                    deep_warned: false,
                    walked: FastSet::default(),
                    spares: Vec::new(),
                }
            }
        };
        self.active = Some(machine);
        self.drive(root_key, node, need)
    }

    pub(super) fn drive(
        &mut self,
        root_key: MachineKey,
        node: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        loop {
            self.memo.evaluation_work += 1;
            if let Some(start) = self.machine_mut()?.passed.take() {
                self.drop_passed(start)?;
                continue;
            }
            if self.machine_mut()?.too_deep {
                let machine = self
                    .active
                    .take()
                    .ok_or(Internal::missed("evaluation machine"))?;
                let mut faults = FastSet::default();
                let mut templates = FastSet::default();
                for frame in &machine.frames {
                    faults.extend(frame.faults.iter().copied());
                    templates.extend(frame.templates.iter().copied());
                }
                let reach: (Vec<u32>, Vec<ScopeId>) = (
                    faults.into_iter().collect(),
                    templates.into_iter().collect(),
                );
                if let Some(Frame {
                    sink: Sink::Into(target),
                    ..
                }) = machine.frames.first()
                {
                    let outer = self.sink;
                    self.edge(&outer, *target);
                }
                for pair in machine.frames.windows(2) {
                    if let [outer, inner] = pair
                        && let Sink::Into(target) = &inner.sink
                    {
                        self.edge(&outer.sink, *target);
                    }
                }
                for frame in machine.frames {
                    let Some(id) = frame.id else {
                        continue;
                    };
                    let done = CallDone {
                        value: CVal::Error,
                        mode: frame.mode,
                        opened: Vec::new(),
                        height: 1,
                        callees: frame.callees,
                    };
                    self.memo.calls.insert(id, done);
                    self.memo.key_reach.insert(id, reach.clone());
                }
                self.reach(reach, node, Ctx::Global)?;
                return Ok(Some(CVal::Error));
            }
            let &(frame, current, ty) = self
                .machine_mut()?
                .work
                .last()
                .ok_or(Internal::missed("evaluation work"))?;
            let memoized = self.frame_value(frame, current, ty)?;
            if memoized.is_some() && !self.ends_checks(frame, current, ty)? {
                self.machine_mut()?.work.pop();
                self.next_start(frame, current, ty)?;
                continue;
            }
            let mut outer = Vec::new();
            let evaluated = match memoized {
                Some(value) => {
                    if !self.check_heads(frame, &mut outer)? {
                        need.extend(outer);
                        let machine = self
                            .active
                            .take()
                            .ok_or(Internal::missed("evaluation machine"))?;
                        self.machines.insert(root_key, machine);
                        return Ok(None);
                    }
                    Some(value)
                }
                None => {
                    self.frame_need.clear();
                    self.frame_pushed = false;
                    let sink = self.frame_sink(frame)?;
                    let enclosing = self.fault_frame.replace(frame);
                    let enclosing_sink = std::mem::replace(&mut self.sink, sink);
                    let evaluated = self.evaluate(current, ty, Ctx::Frame(frame), &mut outer);
                    self.sink = enclosing_sink;
                    self.fault_frame = enclosing;
                    evaluated?
                }
            };
            if self.machine_mut()?.passed.is_some() {
                continue;
            }
            match evaluated {
                Some(value) => {
                    self.machine_mut()?.work.pop();
                    self.store_value(frame, (current, ty), value.clone())?;
                    let record = self.frame(frame)?;
                    let completes = record.root == current && record.ty == ty;
                    self.next_start(frame, current, ty)?;
                    if completes && self.begin_checks(frame, current, ty)? {
                        continue;
                    }
                    let cap = self.limits.nesting_depth.saturating_add(1);
                    let machine = self.machine_mut()?;
                    let record = machine
                        .frames
                        .get_mut(frame)
                        .ok_or(Internal::missed("evaluation frame"))?;
                    if completes && record.run.is_some() {
                        match self.loop_step(frame, value)? {
                            Some(value) => return Ok(Some(value)),
                            None => continue,
                        }
                    }
                    if completes {
                        let (id, call, expected) = (record.id, record.call, record.expected);
                        let id = id.ok_or(Internal::missed("call id"))?;
                        let mut done = machine
                            .frames
                            .pop()
                            .ok_or(Internal::missed("evaluation frame"))?;
                        done.opened.remove(&id);
                        if let Some(parent) = machine.frames.last_mut() {
                            parent.faults.extend(done.faults.iter().copied());
                            parent.templates.extend(done.templates.iter().copied());
                            parent.opened.extend(done.opened.iter().copied());
                            if let Checks::Unchecked | Checks::Done = parent.checks {
                                parent.height =
                                    parent.height.max(done.height.saturating_add(1)).min(cap);
                            }
                        }
                        machine.open_keys.remove(&id);
                        machine.rerun.remove(&id);
                        let sink = match frame.checked_sub(1) {
                            Some(parent) => self.frame_sink(parent)?,
                            None => self.sink,
                        };
                        let enclosing = self.fault_frame;
                        if done.mode == Mode::Untaken {
                            self.fault_frame = frame.checked_sub(1);
                        }
                        let enclosing_sink = std::mem::replace(&mut self.sink, sink);
                        let value = self.call_result(call, current, expected, value);
                        self.sink = enclosing_sink;
                        self.fault_frame = enclosing;
                        let value = value?;
                        let reach: (Vec<u32>, Vec<ScopeId>) = (
                            done.faults.iter().copied().collect(),
                            done.templates.iter().copied().collect(),
                        );
                        let mut opened: Vec<CallId> = done.opened.iter().copied().collect();
                        opened.sort_unstable_by_key(|id| id.0);
                        let entry = CallDone {
                            value: value.clone(),
                            mode: done.mode,
                            opened,
                            height: done.height,
                            callees: std::mem::take(&mut done.callees),
                        };
                        self.machine_mut()?.spares.push(FrameSpare::kept(done));
                        let kept = self.memo.calls.get(&id).is_some_and(|old| {
                            old.mode == Mode::Taken && entry.mode == Mode::Untaken
                        });
                        let reach = if frame == 0 {
                            Some(reach)
                        } else {
                            if !kept {
                                self.memo.key_reach.insert(id, reach);
                            }
                            None
                        };
                        if !kept {
                            self.memo.calls.insert(id, entry);
                        }
                        if let Some(reach) = reach {
                            if !kept {
                                self.memo.key_reach.insert(id, reach.clone());
                            }
                            self.link(&sink, Reachable::Call(id));
                            self.active = None;
                            self.reach(reach, node, Ctx::Global)?;
                            return Ok(Some(value));
                        }
                        self.link(&sink, Reachable::Call(id));
                    }
                }
                None => {
                    if self.frame_pushed {
                        continue;
                    }
                    if !self.frame_need.is_empty() {
                        let machine = self
                            .active
                            .as_mut()
                            .ok_or(Internal::missed("evaluation machine"))?;
                        machine.work.extend(
                            self.frame_need
                                .drain(..)
                                .map(|(node, ty)| (frame, node, ty)),
                        );
                        continue;
                    }
                    need.extend(outer);
                    let machine = self
                        .active
                        .take()
                        .ok_or(Internal::missed("evaluation machine"))?;
                    self.machines.insert(root_key, machine);
                    return Ok(None);
                }
            }
        }
    }

    fn push_frame(
        &mut self,
        call: Call,
        node: NodeId,
        own: EvalTy,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let depth = self
            .machine_mut()?
            .frames
            .last()
            .map_or(0, |frame| frame.depth);
        let repeats = self.machine_mut()?.open_keys.contains(&call.id);
        let caller = self.machine_mut()?.frames.len().saturating_sub(1);
        let mode = self.mode_of(caller)?;
        let chained = self.untaken_depth(caller)?;
        let too_deep = match mode {
            Mode::Taken => repeats || depth >= self.limits.nesting_depth,
            Mode::Untaken => repeats || chained >= self.limits.nesting_depth,
        };
        match (too_deep, mode) {
            (true, Mode::Untaken) if repeats => {
                self.taint(caller, call.id)?;
                return Ok(Some(CVal::Error));
            }
            (true, Mode::Untaken) => {
                self.pass_depth(caller, node, call.function)?;
                return Ok(Some(CVal::Error));
            }
            (true, Mode::Taken) => {
                let names = self.nested_names(call.function)?;
                self.error_at_node(
                    Code::RecursionDepth,
                    node,
                    format!(
                        "the evaluation goes deeper than {} nested calls: {names}",
                        grouped(self.limits.nesting_depth),
                    ),
                )?;
                self.machine_mut()?.too_deep = true;
                return Ok(Some(CVal::Error));
            }
            (false, Mode::Untaken | Mode::Taken) => {}
        }
        if mode == Mode::Taken
            && depth >= self.limits.nesting_depth_warning
            && !self.machine_mut()?.deep_warned
        {
            self.machine_mut()?.deep_warned = true;
            let names = self.nested_names(call.function)?;
            let counted = format!(
                "the evaluation goes deeper than {} nested calls",
                grouped(self.limits.nesting_depth_warning)
            );
            let help = Limit::NestingDepth.warning_help();
            self.placed_at_node(
                Code::LimitWarning,
                node,
                Vec::new(),
                (
                    format!("{}: {names}", warned(&counted, self.limits.nesting_depth)),
                    Some(help),
                ),
                true,
            )?;
        }
        let root = match some!(self.template(&call, node, need)?) {
            Template::Root(root) if self.result_fits(root, own, node, call.function)? => root,
            Template::Reentered => return Ok(Some(CVal::Error)),
            Template::Root(_) | Template::Failed => {
                return self.failed_call(&call, node, Ctx::Frame(caller));
            }
        };
        let defaults = some!(self.default_work(&call, need)?);
        if !self.within_call_limit(node, call.function, call.id)? {
            return Ok(Some(CVal::Error));
        }
        let ty = own;
        let template = self.template_of(self.node(root)?.scope)?;
        let owner = template.map_or(SlotOwner::Root(root), SlotOwner::Template);
        let rerun = self.memo.calls.contains_key(&call.id);
        let sink = match mode {
            Mode::Taken => Sink::Into(Reachable::Call(call.id)),
            Mode::Untaken => Sink::Discard,
        };
        let caller_start = self.frame(caller)?.untaken_start;
        let machine = self.machine_mut()?;
        let FrameSpare {
            values,
            loops,
            faults,
            mut templates,
            opened,
            args,
            mut feeds,
        } = machine.spares.pop().unwrap_or_else(|| {
            CountedWalk::MachineFreshFrame.step();
            FrameSpare::default()
        });
        templates.extend(template);
        feeds.extend_from_slice(&call.feeds);
        let index = machine.frames.len();
        let (untaken_depth, untaken_start) = match mode {
            Mode::Taken => (0, 0),
            Mode::Untaken if chained == 0 => (1, index),
            Mode::Untaken => (chained + 1, caller_start),
        };
        machine.open_keys.insert(call.id);
        if rerun {
            machine.rerun.insert(call.id);
        }
        if let Some(parent) = machine.frames.get_mut(caller) {
            parent.held = Some((node, own, call.id));
        }
        machine.frames.push(Frame {
            id: Some(call.id),
            run: None,
            args: call.values,
            call: node,
            root,
            ty,
            expected: own,
            feeds,
            owner,
            values,
            loops,
            held: None,
            depth: depth.saturating_add(1),
            faults,
            templates,
            checks: Checks::Unchecked,
            mode,
            sink,
            opened,
            untaken_depth,
            untaken_start,
            height: 1,
            callees: Vec::new(),
        });
        machine.work.push((index, root, ty));
        machine
            .work
            .extend(defaults.into_iter().map(|(node, ty)| (index, node, ty)));
        if self.call_scratch.values.capacity() == 0 {
            self.call_scratch.values = args;
        }
        self.frame_pushed = true;
        Ok(None)
    }

    fn failed_call(
        &mut self,
        call: &Call,
        node: NodeId,
        ctx: Ctx,
    ) -> Result<Option<CVal>, Internal> {
        let templates: Vec<ScopeId> = self.template_scope(call.template).into_iter().collect();
        let done = CallDone {
            value: CVal::Error,
            mode: Mode::Taken,
            opened: Vec::new(),
            height: 1,
            callees: Vec::new(),
        };
        self.memo.calls.insert(call.id, done);
        self.memo
            .key_reach
            .insert(call.id, (Vec::new(), templates.clone()));
        self.reach((Vec::new(), templates), node, ctx)?;
        Ok(Some(CVal::Error))
    }

    fn reach_key(&mut self, id: CallId, node: NodeId, ctx: Ctx) -> Result<(), Internal> {
        if let Ctx::Frame(frame) = ctx {
            let Some((faults, templates)) = self.memo.key_reach.get(&id) else {
                return Ok(());
            };
            let record = self
                .active
                .as_mut()
                .ok_or(Internal::missed("evaluation machine"))?
                .frames
                .get_mut(frame)
                .ok_or(Internal::missed("evaluation frame"))?;
            record.faults.extend(faults.iter().copied());
            record.templates.extend(templates.iter().copied());
            return Ok(());
        }
        match self.memo.key_reach.get(&id).cloned() {
            Some(reach) => self.reach(reach, node, ctx),
            None => Ok(()),
        }
    }

    pub(super) fn reach(
        &mut self,
        (faults, templates): (Vec<u32>, Vec<ScopeId>),
        node: NodeId,
        ctx: Ctx,
    ) -> Result<(), Internal> {
        match ctx {
            Ctx::Frame(frame) => {
                let record = self
                    .machine_mut()?
                    .frames
                    .get_mut(frame)
                    .ok_or(Internal::missed("evaluation frame"))?;
                record.faults.extend(faults);
                record.templates.extend(templates);
            }
            Ctx::Global => {
                if self.in_probe(self.node(node)?.scope)? {
                    return Ok(());
                }
                let reactive = faults
                    .iter()
                    .any(|fault| self.memo.reactive_faults.contains(fault));
                if reactive && let Op::Recursive { function, .. } = self.node(node)?.op {
                    self.reactive_body(node, function)?;
                }
                for fault in faults {
                    self.memo.fault_sites.entry(fault).or_default().push(node);
                }
                for template in templates {
                    self.memo
                        .template_sites
                        .entry(template)
                        .or_default()
                        .push(node);
                }
            }
        }
        Ok(())
    }

    fn default_work(
        &mut self,
        call: &Call,
        need: &mut Vec<Query>,
    ) -> Result<Option<Vec<(NodeId, EvalTy)>>, Internal> {
        let defaulted: Vec<usize> = self
            .memo
            .template_keys
            .key(call.template.0)
            .ok_or(Internal::missed("template"))?
            .args
            .iter()
            .enumerate()
            .filter(|(_, shape)| matches!(shape, ArgShape::Default))
            .map(|(index, _)| index)
            .collect();
        if defaulted.is_empty() {
            return Ok(Some(Vec::new()));
        }
        let scope = self
            .template_scope(call.template)
            .ok_or(Internal::missed("template"))?;
        let mut work = Vec::new();
        let mut pending = false;
        for index in defaulted {
            let param = Query::Param(
                scope,
                u32::try_from(index).map_err(|_| Internal::missed("parameter"))?,
            );
            let Some(bound) = self.bound(param, need) else {
                pending = true;
                continue;
            };
            let values: Vec<NodeId> = match &bound {
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
            for node in values {
                let Some(sig) = self.sig(node, need) else {
                    pending = true;
                    continue;
                };
                if !sig.is_error() && !sig.reactive {
                    work.push((node, own_type(&sig, EvalTy::Default)));
                }
            }
        }
        Ok((!pending).then_some(work))
    }

    fn call_result(
        &mut self,
        call: NodeId,
        root: NodeId,
        expected: EvalTy,
        value: CVal,
    ) -> Result<CVal, Internal> {
        if !is_float_ty(expected) {
            return Ok(value);
        }
        let flexible = self.known_sig(root)?.width == Width::Flexible;
        map_leaves(value, &mut FastMap::default(), |leaf| {
            self.convert_integer(call, flexible, leaf, expected)
        })
    }

    fn nested_names(&mut self, last: FnId) -> Result<String, Internal> {
        let mut names: Vec<String> = Vec::new();
        let functions: Vec<FnId> = self
            .active
            .as_ref()
            .ok_or(Internal::missed("evaluation machine"))?
            .frames
            .iter()
            .filter_map(|frame| frame.id)
            .filter_map(|id| self.call_function(id))
            .collect();
        for function in functions.into_iter().chain([last]) {
            let name = self.function_name(function)?;
            if !names.contains(&name) {
                names.push(name);
            }
        }
        Ok(names.join(" → "))
    }

    fn within_call_limit(
        &mut self,
        node: NodeId,
        function: FnId,
        id: CallId,
    ) -> Result<bool, Internal> {
        if self.memo.counted.insert(id, ()).is_some() {
            return Ok(true);
        }
        self.call_count += 1;
        if self.call_count <= self.limits.recursive_calls {
            if !self.call_warning_reported && self.call_count > self.limits.recursive_calls_warning
            {
                self.call_warning_reported = true;
                let counted = format!(
                    "this call of `{}` exceeds {} distinct recursive calls",
                    self.function_name(function)?,
                    grouped(self.limits.recursive_calls_warning)
                );
                let at = {
                    let record = self.node(node)?;
                    (record.loc, record.scope)
                };
                self.warn_limit(
                    Limit::RecursiveCalls,
                    at,
                    (counted, self.limits.recursive_calls),
                )?;
            }
            return Ok(true);
        }
        if !self.call_limit_reported {
            self.call_limit_reported = true;
            let message = format!(
                "this call of `{}` exceeds the {} distinct recursive calls a compilation evaluates",
                self.function_name(function)?,
                grouped(self.limits.recursive_calls)
            );
            let (loc, scope) = {
                let record = self.node(node)?;
                (record.loc, record.scope)
            };
            let help = self.default_calling(function, scope)?;
            self.stop(Code::RecursionLimit, loc, Some(scope), (message, help))?;
        }
        Ok(false)
    }

    pub(super) fn machine_mut(&mut self) -> Result<&mut Machine, Internal> {
        self.active
            .as_mut()
            .ok_or(Internal::missed("evaluation machine"))
    }

    pub(super) fn frame(&self, frame: usize) -> Result<&Frame, Internal> {
        self.active
            .as_ref()
            .ok_or(Internal::missed("evaluation machine"))?
            .frames
            .get(frame)
            .ok_or(Internal::missed("evaluation frame"))
    }

    pub(super) fn frame_value(
        &self,
        frame: usize,
        node: NodeId,
        ty: EvalTy,
    ) -> Result<Option<CVal>, Internal> {
        let record = self.frame(frame)?;
        Ok(match self.frame_slot(record.owner, node)? {
            Some(slot) => record.values.get(slot, ty).cloned(),
            None => None,
        })
    }

    pub(super) fn frame_slot(
        &self,
        owner: SlotOwner,
        node: NodeId,
    ) -> Result<Option<u32>, Internal> {
        if let (Some((template, slot)), SlotOwner::Template(owned)) =
            (self.node(node)?.order, owner)
            && template == owned
        {
            return Ok(Some(slot));
        }
        CountedWalk::FrameForeignSlot.step();
        Ok(self.memo.foreign_slots.get(&(owner, node)).copied())
    }

    pub(super) fn numbered_slot(
        &mut self,
        owner: SlotOwner,
        node: NodeId,
    ) -> Result<u32, Internal> {
        if let Some(slot) = self.frame_slot(owner, node)? {
            return Ok(slot);
        }
        let count = self.memo.slot_counts.entry(owner).or_insert(0);
        let slot = *count;
        *count = count.saturating_add(1);
        self.memo.foreign_slots.insert((owner, node), slot);
        Ok(slot)
    }

    fn store_value(
        &mut self,
        frame: usize,
        (node, ty): (NodeId, EvalTy),
        value: CVal,
    ) -> Result<(), Internal> {
        let owner = self.frame(frame)?.owner;
        let slot = self.numbered_slot(owner, node)?;
        self.frame_mut(frame)?.values.insert(slot, ty, value);
        Ok(())
    }

    pub(super) fn frame_arg(&self, frame: usize, slot: u32) -> Result<CVal, Internal> {
        let slot = usize::try_from(slot).map_err(|_| Internal::missed("argument"))?;
        self.frame(frame)?
            .args
            .get(slot)
            .cloned()
            .ok_or(Internal::missed("argument"))
    }

    pub(in crate::semantics::analysis) fn function_name(
        &self,
        function: FnId,
    ) -> Result<String, Internal> {
        Ok(self
            .hir
            .fn_decl(function)?
            .name
            .as_ref()
            .map_or(String::new(), |name| name.text.clone()))
    }

    pub(in crate::semantics::analysis) fn reactive_recursion(
        &mut self,
        node: NodeId,
        function: FnId,
        (reading, value): (String, bool),
    ) -> Result<(), Internal> {
        let help = Code::ReactiveRecursion
            .help()
            .map(String::from)
            .ok_or(Internal::missed("help"))?;
        let mut chain = vec![format!("`{}`", self.function_name(function)?)];
        let mut related = Vec::new();
        let Some(cycle) = self.call_cycle(function)? else {
            return self.placed_at_node(
                Code::ReactiveRecursion,
                node,
                related,
                (reading, Some(help)),
                value,
            );
        };
        for (step, (caller, call)) in cycle.iter().enumerate() {
            let callee = cycle.get(step + 1).map_or(function, |(next, _)| *next);
            let caller_name = self.function_name(*caller)?;
            let callee_name = self.function_name(callee)?;
            related.push((
                self.hir.expr(*call)?.loc,
                format!("`{caller_name}` calls `{callee_name}` here"),
            ));
            chain.push(format!("`{callee_name}`"));
        }
        let summary = format!(
            "{reading}, and the cycle of calls {} is evaluated by the compiler, so no argument or body on it may read a cell",
            chain.join(" → ")
        );
        self.placed_at_node(
            Code::ReactiveRecursion,
            node,
            related,
            (summary, Some(help)),
            value,
        )
    }

    fn call_cycle(&self, function: FnId) -> Result<Option<Vec<(FnId, ExprId)>>, Internal> {
        let start = function.index();
        let mut parents: BTreeMap<usize, (usize, ExprId)> = BTreeMap::new();
        let mut queue = VecDeque::from([start]);
        while let Some(caller) = queue.pop_front() {
            crate::semantics::counted_walk::CountedWalk::RecursiveCallCycle.step();
            let calls = self
                .collected
                .calls
                .get(caller)
                .ok_or(Internal::missed("function"))?;
            for (target, call) in calls {
                if *target == start {
                    let mut cycle = vec![(caller, *call)];
                    let mut current = caller;
                    while let Some((previous, through)) = parents.get(&current) {
                        crate::semantics::counted_walk::CountedWalk::RecursiveCallCycle.step();
                        cycle.push((*previous, *through));
                        current = *previous;
                    }
                    cycle.reverse();
                    return Ok(Some(
                        cycle
                            .into_iter()
                            .map(|(caller, through)| {
                                (FnId(u32::try_from(caller).unwrap_or(u32::MAX)), through)
                            })
                            .collect(),
                    ));
                }
                let recursive = self
                    .collected
                    .recursive
                    .get(*target)
                    .copied()
                    .unwrap_or(false);
                if !recursive || parents.contains_key(target) {
                    continue;
                }
                parents.insert(*target, (caller, *call));
                queue.push_back(*target);
            }
        }
        Ok(None)
    }
}

fn unit_set_into(feeds: &[Feed], units: &mut Vec<(FamilyId, usize)>) {
    units.clear();
    units.extend(feeds.iter().map(|(family, unit, _)| (*family, *unit)));
    units.sort_by_key(|(family, unit)| (family.index(), *unit));
    units.dedup();
}

fn arg_key(value: &CVal, flexible: bool, ty: EvalTy, family: Option<FamilyId>) -> (ArgKey, ArgSig) {
    let arg_sig = |kind, width| ArgSig {
        kind,
        width,
        family,
        node: ArgReference::Unknown,
    };
    match value {
        CVal::Int(value) if flexible => (
            ArgKey::Int(*value, EvalTy::Exact),
            arg_sig(Kind::Int, Width::Flexible),
        ),
        CVal::Int(value) => {
            let scalar = match ty {
                EvalTy::U8 => symphonyscript_core::Scalar::U8,
                EvalTy::I32 => symphonyscript_core::Scalar::I32,
                _ => symphonyscript_core::Scalar::I64,
            };
            (
                ArgKey::Int(*value, ty),
                arg_sig(Kind::Int, Width::Fixed(scalar)),
            )
        }
        CVal::F32(value) => (
            ArgKey::F32(value.to_bits()),
            arg_sig(Kind::Float, Width::Fixed(symphonyscript_core::Scalar::F32)),
        ),
        CVal::F64(value) => (
            ArgKey::F64(value.to_bits()),
            arg_sig(Kind::Float, Width::Fixed(symphonyscript_core::Scalar::F64)),
        ),
        CVal::Bool(value) => (
            ArgKey::Bool(*value),
            arg_sig(Kind::Bool, Width::Fixed(symphonyscript_core::Scalar::Bool)),
        ),
        CVal::Enum(id, index) => (
            ArgKey::Enum(*id, *index),
            arg_sig(
                Kind::Enum(*id),
                Width::Fixed(symphonyscript_core::Scalar::Enum(*id)),
            ),
        ),
        CVal::Str(text) => (
            ArgKey::Str(text.clone()),
            arg_sig(Kind::Str, Width::Fixed(symphonyscript_core::Scalar::Str)),
        ),
        CVal::Node(item) => (
            ArgKey::Node(*item),
            ArgSig {
                node: ArgReference::Instance(*item),
                ..arg_sig(Kind::Node, Width::Flexible)
            },
        ),
        CVal::Array(_) | CVal::Error => (ArgKey::Error, arg_sig(Kind::Error, Width::Flexible)),
    }
}
