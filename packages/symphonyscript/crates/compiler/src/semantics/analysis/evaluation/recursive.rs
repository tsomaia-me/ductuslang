use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use symphonyscript_core::FamilyId;

use super::checks::Checks;
use super::ctx::Ctx;
use super::frame::Frame;
use super::is_float_ty::is_float_ty;
use super::machine::Machine;
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
use crate::semantics::analysis::call_key::CallKey;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::feed::Feed;
use crate::semantics::analysis::frame_read::FrameRead;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::limit::Limit;
use crate::semantics::analysis::limit_warning::warned;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::template_key::TemplateKey;
use crate::semantics::analysis::val::Val;
use crate::semantics::analysis::width::Width;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::{ExprId, FnId};
use crate::semantics::reporting::grouped;

struct Argument {
    leaves: Vec<(ArgKey, ArgSig)>,
    lengths: Vec<u32>,
    values: Vec<CVal>,
}

impl Argument {
    fn keyed(&self) -> (ArgKey, ArgShape) {
        match (self.lengths.is_empty(), self.leaves.first()) {
            (true, Some((key, sig))) => (key.clone(), ArgShape::Scalar(sig.clone())),
            _ => {
                let (keys, sigs) = self.leaves.iter().cloned().unzip();
                (
                    ArgKey::Array(keys),
                    ArgShape::Array(self.lengths.clone(), sigs),
                )
            }
        }
    }
}

enum Template {
    Root(NodeId),
    Failed,
    Reentered,
}

struct Call {
    key: CallKey,
    function: FnId,
    values: Vec<CVal>,
    shapes: Vec<ArgShape>,
    feeds: Vec<Feed>,
}

impl Engine<'_> {
    pub(super) fn recursive(
        &mut self,
        node: NodeId,
        (function, args): (FnId, Vec<Option<Val>>),
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut shapes = Vec::new();
        let mut missing = false;
        for arg in &args {
            match arg {
                None => {
                    keys.push(ArgKey::Default);
                    shapes.push(ArgShape::Default);
                }
                Some(Val::Scalar(arg)) => match self.argument(*arg, own, ctx, need)? {
                    Some(argument) => {
                        values.extend(argument.values.iter().cloned());
                        let (key, shape) = argument.keyed();
                        keys.push(key);
                        shapes.push(shape);
                    }
                    None => missing = true,
                },
                Some(Val::Array(array)) => {
                    let mut parts = Vec::new();
                    for element in &array.elements {
                        match self.argument(*element, own, ctx, need)? {
                            Some(argument) => parts.push(argument),
                            None => missing = true,
                        }
                    }
                    if missing {
                        continue;
                    }
                    let inner = parts
                        .first()
                        .map(|part| part.lengths.clone())
                        .unwrap_or_default();
                    if parts.iter().any(|part| part.lengths != inner) {
                        self.error_at_node(
                            Code::RaggedArray,
                            node,
                            "the elements of an array are arrays of one length, or all single values",
                        )?;
                        keys.push(ArgKey::Error);
                        continue;
                    }
                    let mut lengths = inner;
                    lengths.extend(array.shape.lengths());
                    let mut element_keys = Vec::new();
                    let mut sigs = Vec::new();
                    for part in parts {
                        for (key, sig) in part.leaves {
                            element_keys.push(key);
                            sigs.push(sig);
                        }
                        values.extend(part.values);
                    }
                    keys.push(ArgKey::Array(element_keys));
                    shapes.push(ArgShape::Array(lengths, sigs));
                }
                Some(Val::Error) => keys.push(ArgKey::Error),
            }
        }
        if missing {
            return Ok(None);
        }
        if keys.contains(&ArgKey::Error) {
            return Ok(Some(CVal::Error));
        }
        let feeds = match ctx {
            Ctx::Global => self.memo.call_feeds.get(&node).cloned().unwrap_or_default(),
            Ctx::Frame(frame) => self.frame(frame)?.feeds.clone(),
        };
        let key = CallKey {
            function,
            args: keys,
            shapes: shapes.clone(),
            expected: own,
            units: unit_set(&feeds),
        };
        if let Some(value) = self.memo.calls.get(&key).cloned() {
            self.reach_key(&key, node, ctx)?;
            return Ok(Some(value));
        }
        let call = Call {
            key,
            function,
            values,
            shapes,
            feeds,
        };
        match ctx {
            Ctx::Global => self.run_machine(call, node, own, need),
            Ctx::Frame(caller) => {
                if self.frame(caller)?.checks.quiet() {
                    if let Some((value, reach)) = self.memo.quiet_calls.get(&call.key).cloned() {
                        self.reach(reach, node, ctx)?;
                        return Ok(Some(value));
                    }
                    if let Some(value) = self.machine_mut()?.tainted_calls.get(&call.key).cloned() {
                        self.taint(caller)?;
                        return Ok(Some(value));
                    }
                }
                self.push_frame(call, node, own, need)
            }
        }
    }

    fn taint(&mut self, frame: usize) -> Result<(), Internal> {
        self.machine_mut()?
            .frames
            .get_mut(frame)
            .ok_or(Internal::missed("evaluation frame"))?
            .tainted = true;
        Ok(())
    }

    fn argument(
        &mut self,
        arg: NodeId,
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<Argument>, Internal> {
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
            let leaves = vec![arg_key(&value, false, EvalTy::Enum(id), None)];
            return Ok(Some(Argument {
                leaves,
                lengths: Vec::new(),
                values: vec![value],
            }));
        }
        if sig.kind == Kind::Bare {
            self.unplaced_variant(arg)?;
            return Ok(Some(Argument {
                leaves: vec![arg_key(&CVal::Error, false, own, None)],
                lengths: Vec::new(),
                values: vec![CVal::Error],
            }));
        }
        let flexible = sig.width == Width::Flexible;
        let ty = if flexible && sig.kind == Kind::Int {
            EvalTy::Exact
        } else {
            own_type(&sig, own)
        };
        let value = some!(self.typed_child(arg, (ty, None), ctx, need)?);
        let (lengths, values) = match &value {
            CVal::Array(array) => (array.lengths(), array.leaves()),
            leaf => (Vec::new(), vec![leaf.clone()]),
        };
        let leaves = values
            .iter()
            .map(|value| arg_key(value, flexible, ty, sig.family))
            .collect();
        Ok(Some(Argument {
            leaves,
            lengths,
            values,
        }))
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
        for shape in shapes {
            let source = match shape {
                ArgShape::Default => ParamSource::Default,
                ArgShape::Scalar(sig) => {
                    let node = self.arg_node(&mut slot, sig, loc, caller)?;
                    ParamSource::Bound(BoundVal::Scalar(Bound::Value(node)))
                }
                ArgShape::Array(lengths, sigs) => {
                    let mut bounds = Vec::with_capacity(sigs.len());
                    for sig in sigs {
                        bounds.push(Bound::Value(self.arg_node(&mut slot, sig, loc, caller)?));
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
        self.call_scope(function, params, loc, caller, true)
    }

    fn template(
        &mut self,
        function: FnId,
        shapes: &[ArgShape],
        feeds: &[Feed],
        call: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Template>, Internal> {
        let key = TemplateKey {
            function,
            args: shapes.to_vec(),
            units: unit_set(feeds),
        };
        let scope = match self.memo.templates.get(&key) {
            Some(scope) => *scope,
            None => {
                let scope = self.instantiate(function, shapes, call)?;
                self.memo.template_feeds.insert(scope, feeds.to_vec());
                self.memo.templates.insert(key, scope);
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
            self.reactive_body(call, function)?;
            return Ok(Some(Template::Failed));
        }
        self.template_units(scope, root)?;
        Ok(Some(Template::Root(root)))
    }

    pub(super) fn frame_read(
        &mut self,
        read: &FrameRead,
        frame: usize,
        (own, partner): (EvalTy, Option<EvalTy>),
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
                self.lacking_choice(read, item)?;
                return Ok(Some(CVal::Error));
            }
        };
        if some!(self.sig(arm, need)).reactive {
            if self.frame(frame)?.checks.quiet() {
                return Ok(Some(CVal::Error));
            }
            let record = self.frame(0)?;
            let (call, function) = (record.call, record.function);
            self.reactive_body(call, function)?;
            return Ok(Some(CVal::Error));
        }
        self.typed_child(arm, (own, partner), ctx, need)
    }

    fn lacking_choice(&mut self, read: &FrameRead, item: ItemRef) -> Result<(), Internal> {
        if self.quiet() || self.may_name(item, &read.name.text)? {
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

    fn frame_heads(&self, frame: usize) -> Result<Vec<NodeId>, Internal> {
        let root = self.frame(frame)?.root;
        let Some(template) = self.template_of(self.node(root)?.scope)? else {
            return Ok(Vec::new());
        };
        Ok(self
            .memo
            .template_heads
            .get(&template)
            .into_iter()
            .flatten()
            .inspect(|_| CountedWalk::RecursiveFrameHeads.step())
            .copied()
            .collect())
    }

    fn begin_checks(&mut self, frame: usize, root: NodeId, ty: EvalTy) -> Result<bool, Internal> {
        if !matches!(self.frame(frame)?.checks, Checks::Unchecked) {
            return Ok(false);
        }
        let mut pending = Vec::new();
        for head in self.frame_heads(frame)? {
            CountedWalk::RecursiveBeginChecks.step();
            let read = self
                .memo
                .frame_reads
                .get(&head)
                .ok_or(Internal::missed("frame read"))?;
            pending.push(read.start);
        }
        let Some(start) = pending.pop() else {
            return Ok(false);
        };
        let machine = self.machine_mut()?;
        machine
            .frames
            .get_mut(frame)
            .ok_or(Internal::missed("evaluation frame"))?
            .checks = Checks::Running { start, pending };
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
        let Checks::Running { start, pending } = &mut record.checks else {
            return Ok(());
        };
        if *start != node || ty != EvalTy::Node {
            return Ok(());
        }
        record.tainted = false;
        machine.tainted_calls.clear();
        if let Some(next) = pending.pop() {
            *start = next;
            machine.work.push((frame, next, EvalTy::Node));
        }
        Ok(())
    }

    fn ends_checks(&self, frame: usize, node: NodeId, ty: EvalTy) -> Result<bool, Internal> {
        let record = self.frame(frame)?;
        Ok(matches!(record.checks, Checks::Running { .. })
            && record.root == node
            && record.ty == ty)
    }

    fn check_heads(&mut self, frame: usize) -> Result<(), Internal> {
        self.machine_mut()?
            .frames
            .get_mut(frame)
            .ok_or(Internal::missed("evaluation frame"))?
            .checks = Checks::Done;
        let enclosing = self.fault_frame.replace(frame);
        let checked = self.checked_heads(frame);
        self.fault_frame = enclosing;
        checked
    }

    fn checked_heads(&mut self, frame: usize) -> Result<(), Internal> {
        for head in self.frame_heads(frame)? {
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

    pub(super) fn quiet_unevaluated(
        &self,
        frame: usize,
        node: NodeId,
        ty: EvalTy,
    ) -> Result<bool, Internal> {
        let query = Query::Eval(node, ty);
        Ok(self.frame(frame)?.checks.quiet()
            && !self.memo.evals.contains_key(&(node, ty))
            && !self.poisoned(query))
    }

    pub(in crate::semantics::analysis) fn quiet(&self) -> bool {
        self.fault_frame
            .zip(self.active.as_ref())
            .and_then(|(frame, machine)| machine.frames.get(frame))
            .is_some_and(|record| record.checks.quiet())
    }

    pub(super) fn reactive_body(&mut self, call: NodeId, function: FnId) -> Result<(), Internal> {
        let reading = format!(
            "the body of the recursive function `{}` reads a cell",
            self.function_name(function)?
        );
        self.reactive_recursion(call, function, reading)
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
        let machine = match self.machines.remove(&call.key) {
            Some(machine) => machine,
            None => {
                let root = match some!(self.template(
                    call.function,
                    &call.shapes,
                    &call.feeds,
                    node,
                    need
                )?) {
                    Template::Root(root) if self.result_fits(root, own, node, call.function)? => {
                        root
                    }
                    Template::Reentered => return Ok(Some(CVal::Error)),
                    Template::Root(_) | Template::Failed => {
                        return self.failed_call(&call, node, Ctx::Global);
                    }
                };
                let defaults = some!(self.default_work(&call, need)?);
                if !self.within_call_limit(node, call.function, false)? {
                    return Ok(Some(CVal::Error));
                }
                let ty = self.body_type(root, own)?;
                let templates = self
                    .template_of(self.node(root)?.scope)?
                    .into_iter()
                    .collect();
                let mut work = vec![(0, root, ty)];
                work.extend(defaults.into_iter().map(|(node, ty)| (0, node, ty)));
                Machine {
                    frames: vec![Frame {
                        key: call.key.clone(),
                        function: call.function,
                        args: call.values,
                        call: node,
                        root,
                        ty,
                        expected: own,
                        feeds: call.feeds,
                        memo: FastMap::default(),
                        depth: 1,
                        faults: FastSet::default(),
                        templates,
                        checks: Checks::Unchecked,
                        tainted: false,
                    }],
                    work,
                    open_keys: [call.key.clone()].into_iter().collect(),
                    too_deep: false,
                    deep_warned: false,
                    tainted_calls: FastMap::default(),
                }
            }
        };
        let root_key = call.key;
        self.active = Some(machine);
        loop {
            self.memo.evaluation_work += 1;
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
                for frame in machine.frames {
                    self.memo.calls.insert(frame.key.clone(), CVal::Error);
                    self.memo.key_reach.insert(frame.key, reach.clone());
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
                    self.check_heads(frame)?;
                    Some(value)
                }
                None => {
                    self.frame_need.clear();
                    self.frame_pushed = false;
                    let enclosing = self.fault_frame.replace(frame);
                    let evaluated = self.evaluate(current, ty, Ctx::Frame(frame), &mut outer);
                    self.fault_frame = enclosing;
                    evaluated?
                }
            };
            match evaluated {
                Some(value) => {
                    let machine = self.machine_mut()?;
                    machine.work.pop();
                    let record = machine
                        .frames
                        .get_mut(frame)
                        .ok_or(Internal::missed("evaluation frame"))?;
                    record.memo.insert((current, ty), value.clone());
                    let completes = record.root == current && record.ty == ty;
                    self.next_start(frame, current, ty)?;
                    if completes && self.begin_checks(frame, current, ty)? {
                        continue;
                    }
                    let machine = self.machine_mut()?;
                    let record = machine
                        .frames
                        .get_mut(frame)
                        .ok_or(Internal::missed("evaluation frame"))?;
                    if completes {
                        let (key, call, expected) =
                            (record.key.clone(), record.call, record.expected);
                        let done = machine
                            .frames
                            .pop()
                            .ok_or(Internal::missed("evaluation frame"))?;
                        if let Some(parent) = machine.frames.last_mut() {
                            parent.faults.extend(done.faults.iter().copied());
                            parent.templates.extend(done.templates.iter().copied());
                            parent.tainted |= done.tainted;
                        }
                        machine.open_keys.remove(&key);
                        let quiet = matches!(done.checks, Checks::Quiet);
                        let enclosing = self.fault_frame;
                        if quiet {
                            self.fault_frame = frame.checked_sub(1);
                        }
                        let value = self.call_result(call, current, expected, value);
                        self.fault_frame = enclosing;
                        let value = value?;
                        let reach: (Vec<u32>, Vec<ScopeId>) = (
                            done.faults.into_iter().collect(),
                            done.templates.into_iter().collect(),
                        );
                        if quiet {
                            if done.tainted {
                                self.machine_mut()?.tainted_calls.insert(key, value);
                            } else {
                                self.memo.quiet_calls.insert(key, (value, reach));
                            }
                            continue;
                        }
                        self.memo.calls.insert(key.clone(), value.clone());
                        self.memo.key_reach.insert(key, reach.clone());
                        if frame == 0 {
                            self.active = None;
                            self.reach(reach, node, Ctx::Global)?;
                            return Ok(Some(value));
                        }
                    }
                }
                None => {
                    if self.frame_pushed {
                        continue;
                    }
                    if !self.frame_need.is_empty() {
                        let pending: Vec<(NodeId, EvalTy)> = std::mem::take(&mut self.frame_need);
                        self.machine_mut()?
                            .work
                            .extend(pending.into_iter().map(|(node, ty)| (frame, node, ty)));
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
        let repeats = self.machine_mut()?.open_keys.contains(&call.key);
        let caller = self.machine_mut()?.frames.len().saturating_sub(1);
        let quiet = self.frame(caller)?.checks.quiet();
        if (repeats || depth >= self.limits.nesting_depth) && quiet {
            self.taint(caller)?;
            return Ok(Some(CVal::Error));
        }
        if repeats || depth >= self.limits.nesting_depth {
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
        if !quiet && depth >= self.limits.nesting_depth_warning && !self.machine_mut()?.deep_warned
        {
            self.machine_mut()?.deep_warned = true;
            let names = self.nested_names(call.function)?;
            let counted = format!(
                "the evaluation goes deeper than {} nested calls",
                grouped(self.limits.nesting_depth_warning)
            );
            self.error_at_node_helped(
                Code::LimitWarning,
                node,
                Vec::new(),
                format!("{}: {names}", warned(&counted, self.limits.nesting_depth)),
                Limit::NestingDepth.warning_help(),
            )?;
        }
        let root =
            match some!(self.template(call.function, &call.shapes, &call.feeds, node, need)?) {
                Template::Root(root) if self.result_fits(root, own, node, call.function)? => root,
                Template::Reentered => return Ok(Some(CVal::Error)),
                Template::Root(_) | Template::Failed => {
                    return self.failed_call(&call, node, Ctx::Frame(caller));
                }
            };
        let defaults = some!(self.default_work(&call, need)?);
        if quiet && self.call_count >= self.limits.recursive_calls {
            self.taint(caller)?;
            return Ok(Some(CVal::Error));
        }
        if !self.within_call_limit(node, call.function, quiet)? {
            return Ok(Some(CVal::Error));
        }
        let ty = self.body_type(root, own)?;
        let templates = self
            .template_of(self.node(root)?.scope)?
            .into_iter()
            .collect();
        let machine = self.machine_mut()?;
        let index = machine.frames.len();
        machine.open_keys.insert(call.key.clone());
        machine.frames.push(Frame {
            key: call.key,
            function: call.function,
            args: call.values,
            call: node,
            root,
            ty,
            expected: own,
            feeds: call.feeds,
            memo: FastMap::default(),
            depth: depth.saturating_add(1),
            faults: FastSet::default(),
            templates,
            checks: if quiet {
                Checks::Quiet
            } else {
                Checks::Unchecked
            },
            tainted: false,
        });
        machine.work.push((index, root, ty));
        machine
            .work
            .extend(defaults.into_iter().map(|(node, ty)| (index, node, ty)));
        self.frame_pushed = true;
        Ok(None)
    }

    fn failed_call(
        &mut self,
        call: &Call,
        node: NodeId,
        ctx: Ctx,
    ) -> Result<Option<CVal>, Internal> {
        let key = TemplateKey {
            function: call.function,
            args: call.shapes.clone(),
            units: unit_set(&call.feeds),
        };
        let templates: Vec<ScopeId> = self.memo.templates.get(&key).copied().into_iter().collect();
        self.memo.calls.insert(call.key.clone(), CVal::Error);
        self.memo
            .key_reach
            .insert(call.key.clone(), (Vec::new(), templates.clone()));
        self.reach((Vec::new(), templates), node, ctx)?;
        Ok(Some(CVal::Error))
    }

    fn reach_key(&mut self, key: &CallKey, node: NodeId, ctx: Ctx) -> Result<(), Internal> {
        match self.memo.key_reach.get(key).cloned() {
            Some(reach) => self.reach(reach, node, ctx),
            None => Ok(()),
        }
    }

    fn reach(
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
        let key = TemplateKey {
            function: call.function,
            args: call.shapes.clone(),
            units: unit_set(&call.feeds),
        };
        let scope = *self
            .memo
            .templates
            .get(&key)
            .ok_or(Internal::missed("template"))?;
        let mut work = Vec::new();
        let mut pending = false;
        for (index, shape) in call.shapes.iter().enumerate() {
            if !matches!(shape, ArgShape::Default) {
                continue;
            }
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

    fn body_type(&self, root: NodeId, expected: EvalTy) -> Result<EvalTy, Internal> {
        let integer = self.known_sig(root)?.kind == Kind::Int;
        Ok(if integer && is_float_ty(expected) {
            EvalTy::Exact
        } else {
            expected
        })
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
            .machine_mut()?
            .frames
            .iter()
            .map(|frame| frame.function)
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
        quiet: bool,
    ) -> Result<bool, Internal> {
        self.call_count += 1;
        if self.call_count <= self.limits.recursive_calls {
            if !quiet
                && !self.call_warning_reported
                && self.call_count > self.limits.recursive_calls_warning
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

    fn machine_mut(&mut self) -> Result<&mut Machine, Internal> {
        self.active
            .as_mut()
            .ok_or(Internal::missed("evaluation machine"))
    }

    fn frame(&self, frame: usize) -> Result<&Frame, Internal> {
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
        Ok(self.frame(frame)?.memo.get(&(node, ty)).cloned())
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
        reading: String,
    ) -> Result<(), Internal> {
        let help = Code::ReactiveRecursion
            .help()
            .map(String::from)
            .ok_or(Internal::missed("help"))?;
        let mut chain = vec![format!("`{}`", self.function_name(function)?)];
        let mut related = Vec::new();
        let Some(cycle) = self.call_cycle(function)? else {
            return self.error_at_node_helped(
                Code::ReactiveRecursion,
                node,
                related,
                reading,
                help,
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
        self.error_at_node_helped(Code::ReactiveRecursion, node, related, summary, help)
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

fn unit_set(feeds: &[Feed]) -> Vec<(FamilyId, usize)> {
    let mut units: Vec<(FamilyId, usize)> = feeds
        .iter()
        .map(|(family, unit, _)| (*family, *unit))
        .collect();
    units.sort_by_key(|(family, unit)| (family.index(), *unit));
    units.dedup();
    units
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
