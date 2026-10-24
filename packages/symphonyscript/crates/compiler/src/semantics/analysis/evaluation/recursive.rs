use std::rc::Rc;

use symphonyscript_core::FamilyId;

use super::ctx::Ctx;
use super::frame::Frame;
use super::is_float_ty::is_float_ty;
use super::machine::Machine;
use super::own_type::own_type;
use crate::diagnostics::Code;
use crate::semantics::analysis::arg_key::ArgKey;
use crate::semantics::analysis::arg_shape::ArgShape;
use crate::semantics::analysis::arg_sig::ArgSig;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::call_key::CallKey;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::feed::Feed;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::template_key::TemplateKey;
use crate::semantics::analysis::val::Val;
use crate::semantics::analysis::width::Width;
use crate::semantics::ir::FnId;

const CALL_LIMIT: usize = 1_000_000;
const DEPTH_LIMIT: usize = 1_024;

struct Argument {
    leaves: Vec<(ArgKey, ArgSig)>,
    lengths: Vec<u32>,
    values: Vec<CVal>,
}

impl Argument {
    fn keyed(&self) -> (ArgKey, ArgShape) {
        match (self.lengths.is_empty(), self.leaves.first()) {
            (true, Some((key, sig))) => (key.clone(), ArgShape::Scalar(*sig)),
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
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Option<CVal> {
        let Op::Recursive { function, args } = self.node(node).op.clone() else {
            return Some(CVal::Error);
        };
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
                Some(Val::Scalar(arg)) => match self.argument(*arg, own, ctx, need) {
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
                        match self.argument(*element, own, ctx, need) {
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
                        );
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
            return None;
        }
        if keys.contains(&ArgKey::Error) {
            return Some(CVal::Error);
        }
        let feeds = match ctx {
            Ctx::Global => self.memo.call_feeds.get(&node).cloned().unwrap_or_default(),
            Ctx::Frame(frame) => self.frame_feeds(frame),
        };
        let key = CallKey {
            function,
            args: keys,
            shapes: shapes.clone(),
            expected: own,
            units: unit_set(&feeds),
        };
        if let Some(value) = self.memo.calls.get(&key) {
            return Some(value.clone());
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
            Ctx::Frame(_) => self.push_frame(call, node, own, need),
        }
    }

    fn argument(
        &mut self,
        arg: NodeId,
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Option<Argument> {
        let sig = self.sig(arg, need)?;
        if sig.kind == Kind::Bare {
            self.unplaced_variant(arg);
            return Some(Argument {
                leaves: vec![arg_key(&CVal::Error, false, own, None)],
                lengths: Vec::new(),
                values: vec![CVal::Error],
            });
        }
        let flexible = sig.width == Width::Flexible;
        let ty = if flexible && sig.kind == Kind::Int {
            EvalTy::Exact
        } else {
            own_type(&sig, own)
        };
        let value = self.child(arg, ty, ctx, need)?;
        let (lengths, values) = match &value {
            CVal::Array(array) => (array.lengths(), array.leaves()),
            leaf => (Vec::new(), vec![leaf.clone()]),
        };
        let leaves = values
            .iter()
            .map(|value| arg_key(value, flexible, ty, sig.family))
            .collect();
        Some(Argument {
            leaves,
            lengths,
            values,
        })
    }

    pub(in crate::semantics::analysis) fn instantiate(
        &mut self,
        function: FnId,
        shapes: &[ArgShape],
        call: NodeId,
    ) -> ScopeId {
        let (loc, caller) = {
            let record = self.node(call);
            (record.loc, record.scope)
        };
        let mut slot = 0u32;
        let mut params = Vec::new();
        for shape in shapes {
            let source = match shape {
                ArgShape::Default => ParamSource::Default,
                ArgShape::Scalar(sig) => {
                    let node = self.arg_node(&mut slot, *sig, loc, caller);
                    ParamSource::Bound(BoundVal::Scalar(Bound::Value(node)))
                }
                ArgShape::Array(lengths, sigs) => {
                    let bounds = sigs
                        .iter()
                        .map(|sig| Bound::Value(self.arg_node(&mut slot, *sig, loc, caller)))
                        .collect();
                    let shape = crate::semantics::analysis::shape::Shape::from_lengths(lengths);
                    ParamSource::Bound(BoundVal::Array(Rc::new((shape, bounds))))
                }
            };
            params.push(source);
        }
        self.call_scope(function, params, loc, caller, true)
    }

    fn template(
        &mut self,
        function: FnId,
        shapes: &[ArgShape],
        feeds: &[Feed],
        call: NodeId,
        need: &mut Vec<Query>,
    ) -> Option<Option<NodeId>> {
        let key = TemplateKey {
            function,
            args: shapes.to_vec(),
            units: unit_set(feeds),
        };
        let scope = match self.memo.templates.get(&key) {
            Some(scope) => *scope,
            None => {
                let scope = self.instantiate(function, shapes, call);
                self.memo.template_feeds.insert(scope, feeds.to_vec());
                self.memo.templates.insert(key, scope);
                scope
            }
        };
        let body = self.hir.fn_decl(function).value;
        let val = self.val(Query::Expr(body, scope), need)?;
        let Some(root) = self.boxed_body(body, scope, &val) else {
            return Some(None);
        };
        let sig = self.sig(root, need)?;
        if sig.reactive {
            self.error_at_node(
                Code::ReactiveRecursion,
                call,
                format!(
                    "the body of the recursive function `{}` reads a cell",
                    self.function_name(function)
                ),
            );
            return Some(None);
        }
        self.template_units(scope, root);
        Some(Some(root))
    }

    fn result_fits(&mut self, root: NodeId, own: EvalTy, call: NodeId, function: FnId) -> bool {
        let kind = self
            .memo
            .sigs
            .get(&root)
            .map_or(Kind::Error, |sig| sig.kind);
        let not_numeric = matches!(
            own,
            EvalTy::Bool | EvalTy::Str | EvalTy::Enum(_) | EvalTy::Node
        );
        let fits = match kind {
            Kind::Float => {
                !not_numeric && !matches!(own, EvalTy::Exact | EvalTy::I32 | EvalTy::I64)
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
                    self.function_name(function)
                ),
            );
        }
        fits
    }

    fn arg_node(
        &mut self,
        slot: &mut u32,
        sig: ArgSig,
        loc: crate::semantics::ir::Loc,
        scope: ScopeId,
    ) -> NodeId {
        let node = self.push_node(Op::Arg(*slot), loc, scope);
        *slot += 1;
        let mut arg_sig = Sig::of(sig.kind, sig.width, false, sig.family);
        arg_sig.frame = true;
        self.memo.arg_sigs.insert(node, arg_sig);
        node
    }

    fn run_machine(
        &mut self,
        call: Call,
        node: NodeId,
        own: EvalTy,
        need: &mut Vec<Query>,
    ) -> Option<CVal> {
        let machine = match self.machines.remove(&call.key) {
            Some(machine) => machine,
            None => {
                let root =
                    match self.template(call.function, &call.shapes, &call.feeds, node, need)? {
                        Some(root) if self.result_fits(root, own, node, call.function) => root,
                        _ => {
                            self.memo.calls.insert(call.key, CVal::Error);
                            return Some(CVal::Error);
                        }
                    };
                if !self.within_call_limit(node, call.function) {
                    return Some(CVal::Error);
                }
                let ty = self.body_type(root, own);
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
                    }],
                    work: vec![(0, root, ty)],
                }
            }
        };
        let root_key = call.key;
        self.active = Some(machine);
        loop {
            let Some(&(frame, current, ty)) =
                self.active.as_ref().and_then(|machine| machine.work.last())
            else {
                self.active = None;
                return Some(CVal::Error);
            };
            if self.frame_value(frame, current, ty).is_some() {
                if let Some(machine) = self.active.as_mut() {
                    machine.work.pop();
                }
                continue;
            }
            self.frame_need.clear();
            self.frame_pushed = false;
            let mut outer = Vec::new();
            match self.evaluate(current, ty, Ctx::Frame(frame), &mut outer) {
                Some(value) => {
                    let Some(machine) = self.active.as_mut() else {
                        return Some(CVal::Error);
                    };
                    machine.work.pop();
                    let Some(record) = machine.frames.get_mut(frame) else {
                        return Some(CVal::Error);
                    };
                    record.memo.insert((current, ty), value.clone());
                    if record.root == current && record.ty == ty {
                        let (key, call, expected) =
                            (record.key.clone(), record.call, record.expected);
                        machine.frames.pop();
                        let value = self.call_result(call, current, expected, value);
                        self.memo.calls.insert(key, value.clone());
                        if frame == 0 {
                            self.active = None;
                            return Some(value);
                        }
                    }
                }
                None => {
                    if self.frame_pushed {
                        continue;
                    }
                    if !self.frame_need.is_empty() {
                        let pending: Vec<(NodeId, EvalTy)> = std::mem::take(&mut self.frame_need);
                        if let Some(machine) = self.active.as_mut() {
                            machine
                                .work
                                .extend(pending.into_iter().map(|(node, ty)| (frame, node, ty)));
                        }
                        continue;
                    }
                    need.extend(outer);
                    if let Some(machine) = self.active.take() {
                        self.machines.insert(root_key, machine);
                    }
                    return None;
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
    ) -> Option<CVal> {
        let depth = self
            .active
            .as_ref()
            .map_or(0, |machine| machine.frames.len());
        if depth >= DEPTH_LIMIT {
            let mut names: Vec<String> = Vec::new();
            let functions: Vec<FnId> = self
                .active
                .as_ref()
                .map(|machine| machine.frames.iter().map(|frame| frame.function).collect())
                .unwrap_or_default();
            for function in functions {
                let name = self.function_name(function);
                if names.last() != Some(&name) {
                    names.push(name);
                }
            }
            self.error_at_node(
                Code::RecursionDepth,
                node,
                format!(
                    "the evaluation goes deeper than 1,024 nested calls: {}",
                    names.join(" → ")
                ),
            );
            return Some(CVal::Error);
        }
        let root = match self.template(call.function, &call.shapes, &call.feeds, node, need)? {
            Some(root) if self.result_fits(root, own, node, call.function) => root,
            _ => {
                self.memo.calls.insert(call.key, CVal::Error);
                return Some(CVal::Error);
            }
        };
        if !self.within_call_limit(node, call.function) {
            return Some(CVal::Error);
        }
        let ty = self.body_type(root, own);
        let Some(machine) = self.active.as_mut() else {
            return Some(CVal::Error);
        };
        let index = machine.frames.len();
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
        });
        machine.work.push((index, root, ty));
        self.frame_pushed = true;
        None
    }

    fn body_type(&self, root: NodeId, expected: EvalTy) -> EvalTy {
        let integer = self
            .memo
            .sigs
            .get(&root)
            .is_some_and(|sig| sig.kind == Kind::Int);
        if integer && is_float_ty(expected) {
            EvalTy::Exact
        } else {
            expected
        }
    }

    fn call_result(&mut self, call: NodeId, root: NodeId, expected: EvalTy, value: CVal) -> CVal {
        if !is_float_ty(expected) {
            return value;
        }
        let flexible = self
            .memo
            .sigs
            .get(&root)
            .is_some_and(|sig| sig.width == Width::Flexible);
        map_leaves(value, |leaf| {
            self.convert_integer(call, flexible, leaf, expected)
        })
    }

    fn within_call_limit(&mut self, node: NodeId, function: FnId) -> bool {
        self.call_count += 1;
        if self.call_count <= CALL_LIMIT {
            return true;
        }
        if !self.call_limit_reported {
            self.call_limit_reported = true;
            self.error_at_node(
                Code::RecursionLimit,
                node,
                format!(
                    "this call of `{}` exceeds the 1,000,000 distinct recursive calls a compilation evaluates",
                    self.function_name(function)
                ),
            );
        }
        false
    }

    pub(super) fn frame_value(&self, frame: usize, node: NodeId, ty: EvalTy) -> Option<CVal> {
        self.active
            .as_ref()?
            .frames
            .get(frame)?
            .memo
            .get(&(node, ty))
            .cloned()
    }

    fn frame_feeds(&self, frame: usize) -> Vec<Feed> {
        self.active
            .as_ref()
            .and_then(|machine| machine.frames.get(frame))
            .map(|record| record.feeds.clone())
            .unwrap_or_default()
    }

    pub(super) fn frame_arg(&self, frame: usize, slot: u32) -> CVal {
        self.active
            .as_ref()
            .and_then(|machine| machine.frames.get(frame))
            .and_then(|record| record.args.get(slot as usize))
            .cloned()
            .unwrap_or(CVal::Error)
    }

    fn function_name(&self, function: FnId) -> String {
        self.hir
            .fn_decl(function)
            .name
            .as_ref()
            .map_or(String::new(), |name| name.text.clone())
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
    };
    match value {
        CVal::Int(value) if flexible => (
            ArgKey::Int(*value, EvalTy::Exact),
            arg_sig(Kind::Int, Width::Flexible),
        ),
        CVal::Int(value) => {
            let scalar = if ty == EvalTy::I32 {
                symphonyscript_core::Scalar::I32
            } else {
                symphonyscript_core::Scalar::I64
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
        CVal::Node(item) => (ArgKey::Node(*item), arg_sig(Kind::Node, Width::Flexible)),
        CVal::Array(_) | CVal::Error => (ArgKey::Error, arg_sig(Kind::Error, Width::Flexible)),
    }
}
