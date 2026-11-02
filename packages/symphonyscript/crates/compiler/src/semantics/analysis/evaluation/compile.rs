use std::cell::OnceCell;
use std::rc::Rc;

use symphonyscript_core::{BinaryOp, UnaryOp, Value};

use super::arg_fact::ArgFact;
use super::call_instr::CallInstr;
use super::concat_part::ConcatPart;
use super::fetch_entry::FetchEntry;
use super::fetched::Fetched;
use super::instr::Instr;
use super::is_float_builtin::is_float_builtin;
use super::is_float_ty::is_float_ty;
use super::is_int_ty::is_int_ty;
use super::lay::Lay;
use super::leaf_fact::LeafFact;
use super::native::native_ty;
use super::of_scalar::of_scalar;
use super::operand_post::OperandPost;
use super::own_type::own_type;
use super::pair::Pair;
use super::pair_type::pair_type;
use super::program::Program;
use super::run::expected_scalar;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::eval_ty_of::eval_ty_of;
use crate::semantics::analysis::fault_state::FaultState;
use crate::semantics::analysis::for_iterable::ForIterable;
use crate::semantics::analysis::ids::NodeId;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::reachable::Reachable;
use crate::semantics::analysis::sig::Sig;
use crate::semantics::analysis::val::Val;
use crate::semantics::analysis::width::Width;
use crate::semantics::builtin::Builtin;
use crate::semantics::counted_walk::CountedWalk;

type LeafPlan = Result<(LeafFact, Option<Req>), Vec<Query>>;

const SAME_ENUM: &str = "`is` compares two numbers, `bool`s, values of one enum or strings";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Site {
    Root,
    Inline,
    Shared,
}

#[derive(Clone, Copy)]
enum Req {
    Typed(NodeId, EvalTy, bool),
    Raw(NodeId, EvalTy),
    Stored(NodeId, EvalTy),
    StoredTyped(NodeId, EvalTy, bool),
}

enum Task {
    Eval(NodeId, EvalTy, Site),
    Set(Vec<Req>, u32),
    Emit(Instr),
    Mark(u32),
}

enum Member {
    Frame(NodeId, EvalTy, Option<EvalTy>, bool),
    Global(Fetched),
}

enum Planned {
    Tasks(Vec<Task>),
    Needs(Vec<Query>),
}

#[derive(Default)]
struct Build {
    code: Vec<Instr>,
    labels: Vec<u32>,
    layouts: Vec<Lay>,
}

impl Build {
    fn label(&mut self) -> u32 {
        self.labels.push(0);
        u32::try_from(self.labels.len() - 1).unwrap_or(u32::MAX)
    }

    fn set(&mut self) -> u32 {
        self.layouts.push(Lay::Plain);
        u32::try_from(self.layouts.len() - 1).unwrap_or(u32::MAX)
    }

    fn lay(&self, lay: Lay) -> Lay {
        match lay {
            Lay::Set(id) => self.layouts.get(id as usize).cloned().unwrap_or(Lay::Plain),
            other => other,
        }
    }

    fn filled(&self, instr: Instr) -> Instr {
        match instr {
            Instr::Element(base, index, lay) => Instr::Element(base, index, self.lay(lay)),
            Instr::Array(node, count, lay) => Instr::Array(node, count, self.lay(lay)),
            Instr::Logic(op, lay) => Instr::Logic(op, self.lay(lay)),
            Instr::Arith(op, node, own, lay) => Instr::Arith(op, node, own, self.lay(lay)),
            Instr::Compare(op, ty, a, b, lay) => Instr::Compare(op, ty, a, b, self.lay(lay)),
            Instr::StrAdd(node, lay) => Instr::StrAdd(node, self.lay(lay)),
            Instr::IntDivFloat(node, own, flexible, lay) => {
                Instr::IntDivFloat(node, own, flexible, self.lay(lay))
            }
            Instr::Div(posts, lay) => Instr::Div(posts, self.lay(lay)),
            Instr::DeferredCheck(node, end, lay, keep) => {
                Instr::DeferredCheck(node, end, self.lay(lay), keep)
            }
            Instr::DeferredSum(node, end, lay, keep) => {
                Instr::DeferredSum(node, end, self.lay(lay), keep)
            }
            Instr::Join(node, lay) => Instr::Join(node, self.lay(lay)),
            Instr::Slice(base, lower, upper, lay) => {
                Instr::Slice(base, lower, upper, self.lay(lay))
            }
            Instr::Rand(args, lay) => Instr::Rand(args, self.lay(lay)),
            Instr::Builtin(node, builtin, own, posts, lay) => {
                Instr::Builtin(node, builtin, own, posts, self.lay(lay))
            }
            Instr::Concat(node, parts, lay) => Instr::Concat(node, parts, self.lay(lay)),
            Instr::Native(node, native, count, lay) => {
                Instr::Native(node, native, count, self.lay(lay))
            }
            Instr::CallWith(mut call) => {
                call.lay = self.lay(call.lay);
                Instr::CallWith(call)
            }
            Instr::ForCheck(range, lay, end) => Instr::ForCheck(range, self.lay(lay), end),
            Instr::ForRun(node, for_node, own, lay) => {
                Instr::ForRun(node, for_node, own, self.lay(lay))
            }
            other => other,
        }
    }

    fn finish(mut self, epoch: u32) -> Program {
        for instr in &mut self.code {
            match instr {
                Instr::NarrowFloat(at)
                | Instr::Jump(at)
                | Instr::BailIfError(at)
                | Instr::MatchLabel(at)
                | Instr::IfGlobal(at)
                | Instr::FlexPair(at)
                | Instr::FromEnd(_, at)
                | Instr::DeferredCheck(_, at, _, _)
                | Instr::DeferredSum(_, at, _, _)
                | Instr::Capacity(_, at)
                | Instr::Gate(at)
                | Instr::LoopStart(_, _, at)
                | Instr::ForCheck(_, _, at) => {
                    *at = self.labels.get(*at as usize).copied().unwrap_or(0);
                }
                Instr::Unless(other, end) => {
                    *other = self.labels.get(*other as usize).copied().unwrap_or(0);
                    *end = self.labels.get(*end as usize).copied().unwrap_or(0);
                }
                Instr::Pick(_, targets, end) => {
                    let resolved: Vec<u32> = targets
                        .iter()
                        .map(|target| self.labels.get(*target as usize).copied().unwrap_or(0))
                        .collect();
                    *targets = Rc::from(resolved);
                    *end = self.labels.get(*end as usize).copied().unwrap_or(0);
                }
                Instr::Push(_)
                | Instr::Held(_)
                | Instr::Arg(_)
                | Instr::Slot(_)
                | Instr::Child(..)
                | Instr::Fetch(_)
                | Instr::Narrow(_)
                | Instr::Fault(..)
                | Instr::Draw(..)
                | Instr::Convert(_)
                | Instr::Fit(..)
                | Instr::Shape(..)
                | Instr::Scaled(..)
                | Instr::ScaledInto(..)
                | Instr::BeforeUnit(..)
                | Instr::Drop
                | Instr::Element(..)
                | Instr::Array(..)
                | Instr::Not
                | Instr::Neg(..)
                | Instr::Logic(..)
                | Instr::Arith(..)
                | Instr::Compare(..)
                | Instr::StrAdd(..)
                | Instr::IntDivFloat(..)
                | Instr::Div(..)
                | Instr::ConvertTo(_)
                | Instr::Join(..)
                | Instr::Slice(..)
                | Instr::Length(_)
                | Instr::Bytes(..)
                | Instr::Rand(..)
                | Instr::Builtin(..)
                | Instr::Concat(..)
                | Instr::FitAnd(..)
                | Instr::Native(..)
                | Instr::CallWith(..)
                | Instr::ForRun(..)
                | Instr::FrameRead(..)
                | Instr::ToRead(..) => {}
            }
        }
        self.code.shrink_to_fit();
        Program {
            code: self.code,
            epoch,
        }
    }
}

impl Engine<'_> {
    pub(super) fn program_for(
        &mut self,
        root: NodeId,
        ty: EvalTy,
    ) -> Result<Result<Rc<Program>, Vec<Query>>, Internal> {
        let cached = !self.cycles_known();
        if cached
            && let Some(program) = self.memo.programs.get(&(root, ty))
            && program.epoch == self.memo.fact_epoch
        {
            return Ok(Ok(program.clone()));
        }
        CountedWalk::ProgramCompile.step();
        let compiled = self.compile(root, ty, cached)?;
        if let Ok(program) = &compiled {
            if self.memo.sigs.get(&root).is_some_and(|sig| sig.frame) {
                CountedWalk::ProgramFrameCompile.step();
            }
            for _ in &program.code {
                CountedWalk::ProgramCode.step();
            }
        }
        if let (true, Ok(program)) = (cached, &compiled) {
            self.memo.programs.insert((root, ty), program.clone());
        }
        Ok(compiled)
    }

    fn compiled_sig(&self, node: NodeId) -> Option<Sig> {
        if self.poisoned(Query::Sig(node)) {
            return Some(Sig::ERROR);
        }
        self.memo.sigs.get(&node).copied()
    }

    fn sigs_of(&self, nodes: &[NodeId]) -> Result<Vec<Sig>, Vec<Query>> {
        let mut sigs = Vec::with_capacity(nodes.len());
        let mut missing = Vec::new();
        for node in nodes {
            match self.compiled_sig(*node) {
                Some(sig) => sigs.push(sig),
                None => missing.push(Query::Sig(*node)),
            }
        }
        if missing.is_empty() {
            Ok(sigs)
        } else {
            Err(missing)
        }
    }

    fn compile(
        &mut self,
        root: NodeId,
        ty: EvalTy,
        cached: bool,
    ) -> Result<Result<Rc<Program>, Vec<Query>>, Internal> {
        let mut build = Build::default();
        let mut tasks = vec![Task::Eval(root, ty, Site::Root)];
        while let Some(task) = tasks.pop() {
            match task {
                Task::Emit(instr) => {
                    let instr = build.filled(instr);
                    build.code.push(instr);
                }
                Task::Mark(label) => {
                    let at =
                        u32::try_from(build.code.len()).map_err(|_| Internal::missed("program"))?;
                    if let Some(slot) = build.labels.get_mut(label as usize) {
                        *slot = at;
                    }
                }
                Task::Set(reqs, id) => {
                    let forward = self.set_tasks(&reqs, id, cached, &mut build)?;
                    tasks.extend(forward.into_iter().rev());
                }
                Task::Eval(node, ty, site) => {
                    let Some(sig) = self.compiled_sig(node) else {
                        if site == Site::Root {
                            return Ok(Err(vec![Query::Sig(node)]));
                        }
                        build.code.push(Instr::Child(node, ty));
                        continue;
                    };
                    match self.plan_node(node, ty, sig, &mut build)? {
                        Planned::Needs(needs) => {
                            if site == Site::Root {
                                return Ok(Err(needs));
                            }
                            build.code.push(Instr::Child(node, ty));
                        }
                        Planned::Tasks(list) => {
                            if site == Site::Shared && !pure(&list) {
                                build.code.push(Instr::Child(node, ty));
                            } else {
                                tasks.extend(list.into_iter().rev());
                            }
                        }
                    }
                }
            }
        }
        Ok(Ok(Rc::new(build.finish(self.memo.fact_epoch))))
    }

    fn request_member(&self, req: Req) -> Member {
        let (node, ty, narrow, stored) = match req {
            Req::Raw(node, ty) => (node, ty, None, false),
            Req::Stored(node, ty) => (node, ty, None, true),
            Req::Typed(node, own, flows) | Req::StoredTyped(node, own, flows) => {
                let Some(sig) = self.compiled_sig(node) else {
                    return Member::Global(Fetched::Typed(node, own, flows));
                };
                let request = if !flows
                    && expected_scalar(own).is_some()
                    && matches!(sig.width, Width::Fixed(_))
                {
                    EvalTy::Default
                } else {
                    own
                };
                let stored = matches!(req, Req::StoredTyped(..));
                if request == own || !sig.is_numeric() {
                    (node, own, None, stored)
                } else {
                    (node, request, Some(own), stored)
                }
            }
        };
        match self.memo.sigs.get(&node) {
            Some(sig) if sig.frame => {
                let stored = stored
                    || self.memo.frame_starts.contains(&node)
                    || self.memo.default_nodes.contains(&node);
                Member::Frame(node, ty, narrow, stored)
            }
            Some(_) | None => Member::Global(Fetched::Raw(node, ty, narrow)),
        }
    }

    fn set_tasks(
        &mut self,
        reqs: &[Req],
        id: u32,
        cached: bool,
        build: &mut Build,
    ) -> Result<Vec<Task>, Internal> {
        let mut frames = Vec::new();
        let mut globals = Vec::new();
        for (index, req) in reqs.iter().enumerate() {
            match self.request_member(*req) {
                Member::Frame(node, ty, narrow, stored) => {
                    frames.push((index, node, ty, narrow, stored));
                }
                Member::Global(fetched) => globals.push((index, fetched)),
            }
        }
        let mut forward = Vec::new();
        let mut emitted = Vec::with_capacity(reqs.len());
        // A frame evaluates an operator's operands in source order (ARCHITECTURE §7
        // stage 6): an untaken frame ends at the first that reaches an open key.
        for &(index, node, ty, narrow, stored) in &frames {
            if stored {
                forward.push(Task::Emit(Instr::Child(node, ty)));
            } else {
                let shared = self
                    .parents
                    .get(node.index())
                    .is_some_and(|parents| *parents > 1);
                let site = if shared { Site::Shared } else { Site::Inline };
                forward.push(Task::Eval(node, ty, site));
            }
            if let Some(own) = narrow {
                forward.push(Task::Emit(Instr::Narrow(own)));
            }
            emitted.push(index);
        }
        if !globals.is_empty() {
            let mut entries = Vec::with_capacity(globals.len());
            for (_, fetched) in &globals {
                entries.push(FetchEntry {
                    fetched: *fetched,
                    settled: self.settled_at_compile(*fetched, cached)?,
                });
            }
            forward.push(Task::Emit(Instr::Fetch(Rc::from(entries))));
            emitted.extend(globals.iter().rev().map(|(index, _)| *index));
        }
        let count = reqs.len();
        let plain = emitted
            .iter()
            .enumerate()
            .all(|(at, index)| *index + at + 1 == count);
        let lay = if plain {
            Lay::Plain
        } else {
            let mut positions = vec![0u32; count];
            for (at, index) in emitted.iter().enumerate() {
                if let Some(slot) = positions.get_mut(*index) {
                    *slot = u32::try_from(at).unwrap_or(u32::MAX);
                }
            }
            Lay::At(Rc::new(positions))
        };
        if let Some(slot) = build.layouts.get_mut(id as usize) {
            *slot = lay;
        }
        Ok(forward)
    }

    fn settled_at_compile(
        &self,
        fetched: Fetched,
        cached: bool,
    ) -> Result<OnceCell<CVal>, Internal> {
        let settled = OnceCell::new();
        let Fetched::Raw(node, ty, narrow) = fetched else {
            return Ok(settled);
        };
        if !cached || self.memo.sigs.get(&node).is_none_or(|sig| sig.frame) {
            return Ok(settled);
        }
        if matches!(
            self.memo.recorded.get(&Reachable::Eval(node, ty)),
            Some(FaultState::Recorded { .. })
        ) {
            return Ok(settled);
        }
        if let Some(value) = self.memo.evals.get(&(node, ty)) {
            let value = match narrow {
                Some(own) => super::run::narrowed(value.clone(), own)?,
                None => value.clone(),
            };
            let _ = settled.set(value);
        }
        Ok(settled)
    }

    fn plan_node(
        &mut self,
        node: NodeId,
        ty: EvalTy,
        sig: Sig,
        build: &mut Build,
    ) -> Result<Planned, Internal> {
        if sig.is_error() || sig.reactive || self.memo.kept_types.contains(&node) {
            return Ok(Planned::Tasks(vec![Task::Emit(Instr::Push(CVal::Error))]));
        }
        let own = self.expected_own(node, &sig, ty)?;
        let flows = own == ty && expected_scalar(ty).is_some();
        let scale = self.memo.default_scales.get(&node).copied();
        let shape = self
            .memo
            .shapes
            .get(&node)
            .filter(|need| !need.by_taken_arm)
            .map(|need| Rc::new(need.clone()));
        let mut list = Vec::new();
        if let (Width::Widened(integer), Some(scale)) = (sig.width, scale) {
            match self.plan_value(node, (of_scalar(integer), false), build)? {
                Planned::Needs(needs) => return Ok(Planned::Needs(needs)),
                Planned::Tasks(tasks) => list.extend(tasks),
            }
            list.push(Task::Emit(Instr::ScaledInto(scale, own, ty)));
            if let Some(shape) = shape {
                list.push(Task::Emit(Instr::Shape(node, shape)));
            }
            return Ok(Planned::Tasks(list));
        }
        if own == EvalTy::Default && sig.kind == Kind::Float {
            let wide = build.set();
            list.push(Task::Set(vec![Req::Raw(node, EvalTy::DefaultF64)], wide));
            let skip = build.label();
            list.push(Task::Emit(Instr::NarrowFloat(skip)));
            let narrow = build.set();
            list.push(Task::Set(vec![Req::Raw(node, EvalTy::DefaultF32)], narrow));
            list.push(Task::Mark(skip));
        } else {
            match self.plan_value(node, (own, flows), build)? {
                Planned::Needs(needs) => return Ok(Planned::Needs(needs)),
                Planned::Tasks(tasks) => list.extend(tasks),
            }
            let reads_use = matches!(self.node(node)?.op, Op::Use(_));
            let literal = matches!(self.node(node)?.op, Op::Number(..));
            if let Some(scale) = scale {
                if reads_use {
                    list.push(Task::Emit(Instr::BeforeUnit(node, ty)));
                }
                if !literal {
                    let target = if is_float_ty(ty) { ty } else { own };
                    list.push(Task::Emit(Instr::Scaled(node, scale, target)));
                }
            }
        }
        if sig.kind == Kind::Int
            && sig.width == Width::Flexible
            && super::int_range::int_range(ty).is_some()
            && self.takes_a_type(node)?
        {
            if let Some(shape) = shape {
                list.push(Task::Emit(Instr::Shape(node, shape)));
            }
            list.push(Task::Emit(Instr::Fit(node, ty)));
            return Ok(Planned::Tasks(list));
        }
        if converts(sig.kind, ty) {
            list.push(Task::Emit(Instr::Convert(ty)));
        }
        if let Some(shape) = shape {
            list.push(Task::Emit(Instr::Shape(node, shape)));
        }
        Ok(Planned::Tasks(list))
    }

    fn plan_value(
        &mut self,
        node: NodeId,
        (own, flows): (EvalTy, bool),
        build: &mut Build,
    ) -> Result<Planned, Internal> {
        let Some(read) = self.memo.frame_reads.get(&node).cloned() else {
            return self.plan_op(node, (own, flows), build);
        };
        let global = build.label();
        let end = build.label();
        let start = build.set();
        let op = match self.plan_op(node, (own, flows), build)? {
            Planned::Needs(needs) => return Ok(Planned::Needs(needs)),
            Planned::Tasks(op) => op,
        };
        let mut list = vec![
            Task::Emit(Instr::IfGlobal(global)),
            Task::Set(vec![Req::Stored(read.start, EvalTy::Node)], start),
            Task::Emit(Instr::FrameRead(read, own, flows)),
            Task::Emit(Instr::Jump(end)),
            Task::Mark(global),
        ];
        list.extend(op);
        list.push(Task::Mark(end));
        Ok(Planned::Tasks(list))
    }

    fn plan_op(
        &mut self,
        node: NodeId,
        (own, flows): (EvalTy, bool),
        build: &mut Build,
    ) -> Result<Planned, Internal> {
        let typed = |at: NodeId| Req::Typed(at, own, flows);
        let op = self.node(node)?.op.clone();
        let mut list = Vec::new();
        match op {
            Op::Read(_) | Op::History { .. } | Op::Stepped(_) => {
                list.push(Task::Emit(Instr::Push(CVal::Error)));
            }
            Op::Join(lhs, rhs) => {
                let set = build.set();
                list.push(Task::Set(vec![typed(lhs), typed(rhs)], set));
                list.push(Task::Emit(Instr::Join(node, Lay::Set(set))));
            }
            Op::Slice { base, lower, upper } => {
                let mut reqs = vec![typed(base)];
                for (bound, _) in lower.iter().chain(upper.iter()) {
                    let sig = match self.sigs_of(&[*bound]) {
                        Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                        Err(needs) => return Ok(Planned::Needs(needs)),
                    };
                    reqs.push(Req::Raw(*bound, own_type(&sig, EvalTy::Exact)));
                }
                let set = build.set();
                list.push(Task::Set(reqs, set));
                list.push(Task::Emit(Instr::Slice(base, lower, upper, Lay::Set(set))));
            }
            Op::Native { native, args } => return self.plan_native(node, native, &args, build),
            Op::Builtin(Builtin::Len, args) => {
                let operand = *args.first().ok_or(Internal::missed("len operand"))?;
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(operand, EvalTy::Default)], set));
                list.push(Task::Emit(Instr::Length(operand)));
            }
            Op::Builtin(Builtin::Bytes, args) => {
                let operand = *args.first().ok_or(Internal::missed("bytes operand"))?;
                let sig = match self.sigs_of(&[operand]) {
                    Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                let ty = if sig.kind == Kind::Str {
                    EvalTy::Str
                } else {
                    own_type(&sig, EvalTy::Exact)
                };
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(operand, ty)], set));
                list.push(Task::Emit(Instr::Bytes(node, operand)));
            }
            Op::Builtin(Builtin::Capacity, args) => {
                let operand = *args.first().ok_or(Internal::missed("capacity operand"))?;
                let end = build.label();
                list.push(Task::Emit(Instr::Capacity(operand, end)));
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(operand, EvalTy::Default)], set));
                list.push(Task::Emit(Instr::Length(operand)));
                list.push(Task::Mark(end));
            }
            Op::Pick { index, items } => {
                let sig = match self.sigs_of(&[index]) {
                    Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                let set = build.set();
                list.push(Task::Set(
                    vec![Req::Raw(index, own_type(&sig, EvalTy::Exact))],
                    set,
                ));
                let end = build.label();
                let targets: Vec<u32> = items.iter().map(|_| build.label()).collect();
                list.push(Task::Emit(Instr::Pick(
                    index,
                    Rc::from(targets.clone()),
                    end,
                )));
                for (item, target) in items.iter().zip(targets) {
                    list.push(Task::Mark(target));
                    let set = build.set();
                    list.push(Task::Set(vec![typed(*item)], set));
                    list.push(Task::Emit(Instr::Jump(end)));
                }
                list.push(Task::Mark(end));
            }
            Op::Element { base, index } => {
                let sig = match self.sigs_of(&[index]) {
                    Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                let set = build.set();
                list.push(Task::Set(
                    vec![typed(base), Req::Raw(index, own_type(&sig, EvalTy::Exact))],
                    set,
                ));
                list.push(Task::Emit(Instr::Element(base, index, Lay::Set(set))));
            }
            Op::Array(elements) => {
                let set = build.set();
                let count = u32::try_from(elements.len()).map_err(|_| Internal::missed("array"))?;
                list.push(Task::Set(
                    elements.iter().map(|at| typed(*at)).collect(),
                    set,
                ));
                list.push(Task::Emit(Instr::Array(node, count, Lay::Set(set))));
            }
            Op::Arg(slot) => list.push(Task::Emit(Instr::Arg(slot))),
            Op::Slot(slot) => list.push(Task::Emit(Instr::Slot(slot))),
            Op::For(for_node) => return self.plan_for(node, &for_node, (own, flows), build),
            Op::Number(number, suffix) => {
                list.push(Task::Emit(
                    match self.literal_value(node, &number, suffix, own)? {
                        Ok(value) => Instr::Push(value),
                        Err((code, message)) => Instr::Fault(code, node, Rc::from(message)),
                    },
                ));
            }
            Op::Const(value) => {
                list.push(Task::Emit(Instr::Push(match value {
                    Value::U8(value) => CVal::Int(i128::from(value)),
                    Value::I32(value) => CVal::Int(i128::from(value)),
                    Value::I64(value) => CVal::Int(i128::from(value)),
                    Value::F32(value) => CVal::F32(value),
                    Value::F64(value) => CVal::F64(value),
                    Value::Bool(value) => CVal::Bool(value),
                    Value::Enum(id, index) => CVal::Enum(id, index),
                    Value::Str(_) => CVal::Error,
                })));
            }
            Op::Use(target) | Op::Chosen(target) => {
                let set = build.set();
                list.push(Task::Set(vec![typed(target)], set));
            }
            Op::Bool(value) => list.push(Task::Emit(Instr::Push(CVal::Bool(value)))),
            Op::Str(text) => {
                list.push(Task::Emit(Instr::Push(CVal::Str(Rc::from(text.as_str())))));
                let held = u64::try_from(text.chars().count()).unwrap_or(u64::MAX);
                if held
                    > self
                        .limits
                        .value_elements_warning
                        .min(self.limits.value_elements)
                {
                    list.push(Task::Emit(Instr::Held(node)));
                }
            }
            Op::Variant(id, index) => list.push(Task::Emit(Instr::Push(CVal::Enum(id, index)))),
            Op::NodeRef(item) => list.push(Task::Emit(Instr::Push(CVal::Node(item)))),
            Op::Pi => {
                list.push(Task::Emit(Instr::Push(if super::is_f32::is_f32(own) {
                    CVal::F32(std::f32::consts::PI)
                } else {
                    CVal::F64(std::f64::consts::PI)
                })));
            }
            Op::Bare(name) => list.push(Task::Emit(self.bare_value(node, &name, own)?)),
            Op::Concat(pieces) => {
                let mut values = Vec::new();
                let mut parts = Vec::with_capacity(pieces.len());
                for piece in &pieces {
                    match piece {
                        crate::semantics::analysis::piece::Piece::Text(text) => {
                            parts.push(ConcatPart::Text(Rc::from(text.as_str())));
                        }
                        crate::semantics::analysis::piece::Piece::Value(value) => {
                            values.push(*value);
                        }
                    }
                }
                let sigs = match self.sigs_of(&values) {
                    Ok(sigs) => sigs,
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                let mut reqs = Vec::with_capacity(values.len());
                let mut at = 0;
                parts.clear();
                for piece in &pieces {
                    match piece {
                        crate::semantics::analysis::piece::Piece::Text(text) => {
                            parts.push(ConcatPart::Text(Rc::from(text.as_str())));
                        }
                        crate::semantics::analysis::piece::Piece::Value(value) => {
                            let sig = sigs.get(at).copied().unwrap_or(Sig::ERROR);
                            at += 1;
                            reqs.push(Req::Raw(*value, own_type(&sig, EvalTy::Default)));
                            parts.push(ConcatPart::Value(*value, self.fits_by_default(*value)));
                        }
                    }
                }
                let set = build.set();
                list.push(Task::Set(reqs, set));
                list.push(Task::Emit(Instr::Concat(
                    node,
                    Rc::from(parts),
                    Lay::Set(set),
                )));
            }
            Op::Unary(UnaryOp::Not, operand) => {
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(operand, EvalTy::Bool)], set));
                list.push(Task::Emit(Instr::Not));
            }
            Op::Unary(UnaryOp::Neg, operand) => {
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(operand, own)], set));
                list.push(Task::Emit(Instr::Neg(node, own)));
            }
            Op::Binary(op, lhs, rhs) => {
                return self.plan_binary(node, (op, lhs, rhs), (own, flows), build);
            }
            Op::If(condition, then, otherwise) => {
                let (other, end) = (build.label(), build.label());
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(condition, EvalTy::Bool)], set));
                list.push(Task::Emit(Instr::Unless(other, end)));
                let set = build.set();
                list.push(Task::Set(vec![typed(then)], set));
                list.push(Task::Emit(Instr::Jump(end)));
                list.push(Task::Mark(other));
                let set = build.set();
                list.push(Task::Set(vec![typed(otherwise)], set));
                list.push(Task::Mark(end));
            }
            Op::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let sig = match self.sigs_of(&[scrutinee]) {
                    Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                let labels: Vec<NodeId> = arms.iter().map(|(label, _)| *label).collect();
                let Some(choice) = self.choice_ty(scrutinee, &sig, &labels)? else {
                    list.push(Task::Emit(Instr::Push(CVal::Error)));
                    return Ok(Planned::Tasks(list));
                };
                if let Err(needs) = self.sigs_of(&labels) {
                    return Ok(Planned::Needs(needs));
                }
                let end = build.label();
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(scrutinee, choice)], set));
                if self.fits_by_default(scrutinee) {
                    list.push(Task::Emit(Instr::Fit(scrutinee, EvalTy::I64)));
                }
                list.push(Task::Emit(Instr::BailIfError(end)));
                let targets: Vec<u32> = arms.iter().map(|_| build.label()).collect();
                for (label, target) in labels.iter().zip(&targets) {
                    let set = build.set();
                    list.push(Task::Set(vec![Req::Raw(*label, choice)], set));
                    if self.fits_by_default(*label) {
                        list.push(Task::Emit(Instr::Fit(*label, EvalTy::I64)));
                    }
                    list.push(Task::Emit(Instr::MatchLabel(*target)));
                }
                list.push(Task::Emit(Instr::Drop));
                match otherwise {
                    Some(arm) => {
                        let set = build.set();
                        list.push(Task::Set(vec![typed(arm)], set));
                    }
                    None => list.push(Task::Emit(Instr::Push(CVal::Error))),
                }
                list.push(Task::Emit(Instr::Jump(end)));
                for ((_, arm), target) in arms.iter().zip(targets) {
                    list.push(Task::Mark(target));
                    let set = build.set();
                    list.push(Task::Set(vec![typed(*arm)], set));
                    list.push(Task::Emit(Instr::Jump(end)));
                }
                list.push(Task::Mark(end));
            }
            Op::Convert(_, operand) => {
                let sig = match self.sigs_of(&[operand]) {
                    Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                if !sig.is_numeric() {
                    list.push(Task::Emit(Instr::Push(CVal::Error)));
                    return Ok(Planned::Tasks(list));
                }
                let operand_ty = if is_float_ty(own) {
                    own
                } else {
                    EvalTy::Default
                };
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(operand, operand_ty)], set));
                if is_int_ty(own) && self.fits_by_default(operand) {
                    list.push(Task::Emit(Instr::Fit(operand, EvalTy::I64)));
                }
                list.push(Task::Emit(Instr::ConvertTo(own)));
            }
            Op::Annotate(_, operand) => {
                let sig = match self.sigs_of(&[operand]) {
                    Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                let set = build.set();
                list.push(Task::Set(vec![Req::Raw(operand, own)], set));
                if sig.width == Width::Flexible {
                    list.push(Task::Emit(Instr::Fit(node, own)));
                }
            }
            Op::Builtin(builtin, args) => {
                return self.plan_builtin(node, builtin, &args, (own, flows), build);
            }
            Op::Recursive { function, args } => {
                return self.plan_call(node, function, &args, own, build);
            }
            Op::ToRead { wire, attrs, .. } => {
                let (item, attr) = *attrs.first().ok_or(Internal::missed("attribute"))?;
                list.push(Task::Emit(Instr::ToRead(wire, item, attr, own)));
            }
        }
        Ok(Planned::Tasks(list))
    }

    fn fits_by_default(&self, node: NodeId) -> bool {
        self.memo
            .sigs
            .get(&node)
            .is_some_and(|sig| sig.kind == Kind::Int && sig.width == Width::Flexible)
    }

    fn bare_value(&self, node: NodeId, name: &str, own: EvalTy) -> Result<Instr, Internal> {
        let enumeration = match own {
            EvalTy::Enum(id) => id,
            EvalTy::Exact
            | EvalTy::Default
            | EvalTy::U8
            | EvalTy::I32
            | EvalTy::I64
            | EvalTy::F32
            | EvalTy::F64
            | EvalTy::DefaultF32
            | EvalTy::DefaultF64
            | EvalTy::Bool
            | EvalTy::Str
            | EvalTy::Node => {
                return Ok(Instr::Push(match self.memo.bare_enums.get(&node) {
                    Some(id) => match self.variants.position(*id, name)? {
                        Some(index) => CVal::Enum(*id, index),
                        None => CVal::Error,
                    },
                    None => CVal::Error,
                }));
            }
        };
        Ok(match self.variants.position(enumeration, name)? {
            Some(index) => Instr::Push(CVal::Enum(enumeration, index)),
            None if self.in_name_probe(self.node(node)?.scope)? => Instr::Push(CVal::Error),
            None => {
                let enum_name = self.enum_decl(enumeration)?.name.clone();
                Instr::Fault(
                    Code::UnknownName,
                    node,
                    Rc::from(format!("`{name}` is not a variant of `{enum_name}`")),
                )
            }
        })
    }

    fn plan_binary(
        &mut self,
        node: NodeId,
        (op, lhs, rhs): (BinaryOp, NodeId, NodeId),
        (own, flows): (EvalTy, bool),
        build: &mut Build,
    ) -> Result<Planned, Internal> {
        let mut list = Vec::new();
        let end = build.label();
        let from_end = self.memo.from_end_indexes.contains(&node);
        let deferred = self.memo.deferred_sums.contains(&node);
        let arithmetic = match op {
            BinaryOp::Add => own != EvalTy::Str,
            BinaryOp::IntDiv => !is_float_ty(own),
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Rem | BinaryOp::Pow => true,
            BinaryOp::Div
            | BinaryOp::And
            | BinaryOp::Or
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Is
            | BinaryOp::IsNot => false,
        };
        let kept = deferred && arithmetic && !from_end;
        let again = from_end || deferred && !kept;
        let raw = |at: NodeId, ty: EvalTy| {
            if again {
                Req::Stored(at, ty)
            } else {
                Req::Raw(at, ty)
            }
        };
        let typed = |at: NodeId, ty: EvalTy, flows: bool| {
            if again {
                Req::StoredTyped(at, ty, flows)
            } else {
                Req::Typed(at, ty, flows)
            }
        };
        if from_end {
            let sig = match self.sigs_of(&[rhs]) {
                Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                Err(needs) => return Ok(Planned::Needs(needs)),
            };
            let set = build.set();
            list.push(Task::Set(
                vec![Req::Stored(rhs, own_type(&sig, EvalTy::Exact))],
                set,
            ));
            list.push(Task::Emit(Instr::FromEnd(rhs, end)));
        }
        if kept {
            let set = build.set();
            list.push(Task::Set(
                vec![typed(lhs, own, flows), typed(rhs, own, flows)],
                set,
            ));
            list.push(Task::Emit(if op == BinaryOp::Add {
                Instr::DeferredSum(node, end, Lay::Set(set), true)
            } else {
                Instr::DeferredCheck(node, end, Lay::Set(set), true)
            }));
            list.push(Task::Emit(Instr::Arith(op, node, own, Lay::Set(set))));
            list.push(Task::Mark(end));
            return Ok(Planned::Tasks(list));
        }
        if deferred {
            let set = build.set();
            list.push(Task::Set(
                vec![typed(lhs, own, flows), typed(rhs, own, flows)],
                set,
            ));
            list.push(Task::Emit(if op == BinaryOp::Add {
                Instr::DeferredSum(node, end, Lay::Set(set), false)
            } else {
                Instr::DeferredCheck(node, end, Lay::Set(set), false)
            }));
        }
        let set = build.set();
        match op {
            BinaryOp::And | BinaryOp::Or => {
                list.push(Task::Set(
                    vec![raw(lhs, EvalTy::Bool), raw(rhs, EvalTy::Bool)],
                    set,
                ));
                list.push(Task::Emit(Instr::Logic(op, Lay::Set(set))));
            }
            BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Is
            | BinaryOp::IsNot => {
                let sigs = match self.sigs_of(&[lhs, rhs]) {
                    Ok(sigs) => sigs,
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                let (a_sig, b_sig) = (
                    sigs.first().copied().unwrap_or(Sig::ERROR),
                    sigs.get(1).copied().unwrap_or(Sig::ERROR),
                );
                if matches!(op, BinaryOp::Is | BinaryOp::IsNot) && self.enums_cross(lhs, rhs)? {
                    list.push(Task::Emit(Instr::Fault(
                        Code::OperandType,
                        node,
                        Rc::from(SAME_ENUM),
                    )));
                    list.push(Task::Mark(end));
                    return Ok(Planned::Tasks(list));
                }
                let carried = match (a_sig.kind, b_sig.kind) {
                    (Kind::Bare, Kind::Bare) => {
                        self.carried_enum_of(lhs)?.or(self.carried_enum_of(rhs)?)
                    }
                    _ => None,
                };
                let pair = carried.map_or_else(
                    || pair_type(&a_sig, &b_sig),
                    |id| Pair::Ty(EvalTy::Enum(id)),
                );
                match pair {
                    Pair::Ty(ty) => {
                        list.push(Task::Set(
                            vec![typed(lhs, ty, false), typed(rhs, ty, false)],
                            set,
                        ));
                        list.push(Task::Emit(Instr::Compare(
                            op,
                            fit_ty(ty),
                            operand_fit(lhs, &a_sig),
                            operand_fit(rhs, &b_sig),
                            Lay::Set(set),
                        )));
                    }
                    Pair::FlexibleFloat => {
                        list.push(Task::Set(
                            vec![
                                Req::Stored(lhs, EvalTy::DefaultF64),
                                Req::Stored(rhs, EvalTy::DefaultF64),
                            ],
                            set,
                        ));
                        let narrow = build.label();
                        list.push(Task::Emit(Instr::FlexPair(narrow)));
                        for ty in [EvalTy::DefaultF64, EvalTy::DefaultF32] {
                            if ty == EvalTy::DefaultF32 {
                                list.push(Task::Mark(narrow));
                            }
                            let set = build.set();
                            list.push(Task::Set(
                                vec![
                                    Req::StoredTyped(lhs, ty, false),
                                    Req::StoredTyped(rhs, ty, false),
                                ],
                                set,
                            ));
                            list.push(Task::Emit(Instr::Compare(
                                op,
                                fit_ty(ty),
                                operand_fit(lhs, &a_sig),
                                operand_fit(rhs, &b_sig),
                                Lay::Set(set),
                            )));
                            list.push(Task::Emit(Instr::Jump(end)));
                        }
                    }
                }
            }
            BinaryOp::Add if own == EvalTy::Str => {
                list.push(Task::Set(
                    vec![raw(lhs, EvalTy::Str), raw(rhs, EvalTy::Str)],
                    set,
                ));
                list.push(Task::Emit(Instr::StrAdd(node, Lay::Set(set))));
            }
            BinaryOp::IntDiv if is_float_ty(own) => {
                list.push(Task::Set(
                    vec![raw(lhs, EvalTy::Default), raw(rhs, EvalTy::Default)],
                    set,
                ));
                let flexible = self
                    .memo
                    .sigs
                    .get(&node)
                    .is_some_and(|sig| !matches!(sig.width, Width::Fixed(_)));
                list.push(Task::Emit(Instr::IntDivFloat(
                    node,
                    own,
                    flexible,
                    Lay::Set(set),
                )));
            }
            BinaryOp::Div => {
                let (Some((a, a_post)), Some((b, b_post))) = (
                    self.float_plan(lhs, (own, flows), again),
                    self.float_plan(rhs, (own, flows), again),
                ) else {
                    return Ok(Planned::Needs(
                        [lhs, rhs]
                            .into_iter()
                            .filter(|at| self.compiled_sig(*at).is_none())
                            .map(Query::Sig)
                            .collect(),
                    ));
                };
                list.push(Task::Set(vec![a, b], set));
                list.push(Task::Emit(Instr::Div(
                    Rc::new((a_post, b_post)),
                    Lay::Set(set),
                )));
            }
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::IntDiv
            | BinaryOp::Rem
            | BinaryOp::Pow => {
                list.push(Task::Set(
                    vec![typed(lhs, own, flows), typed(rhs, own, flows)],
                    set,
                ));
                list.push(Task::Emit(Instr::Arith(op, node, own, Lay::Set(set))));
            }
        }
        list.push(Task::Mark(end));
        Ok(Planned::Tasks(list))
    }

    fn float_plan(
        &self,
        operand: NodeId,
        (own, flows): (EvalTy, bool),
        again: bool,
    ) -> Option<(Req, OperandPost)> {
        let sig = self.compiled_sig(operand)?;
        let raw = |ty: EvalTy| {
            if again {
                Req::Stored(operand, ty)
            } else {
                Req::Raw(operand, ty)
            }
        };
        if sig.kind != Kind::Int || !is_float_ty(own) {
            let typed = if again {
                Req::StoredTyped(operand, own, flows)
            } else {
                Req::Typed(operand, own, flows)
            };
            return Some((typed, OperandPost::Keep));
        }
        let flexible = sig.width == Width::Flexible;
        let widens_into_own = match (sig.width, expected_scalar(own)) {
            (Width::Fixed(scalar), Some(target)) => {
                crate::semantics::analysis::widens::widens(scalar, target)
            }
            _ => false,
        };
        if flexible && !super::is_chosen_float::is_chosen_float(own) || flows && widens_into_own {
            return Some((raw(own), OperandPost::Keep));
        }
        let ty = own_type(&sig, EvalTy::Exact);
        if flexible && super::int_range::int_range(ty).is_some() {
            return Some((raw(ty), OperandPost::FitThenFloat(operand, ty, own)));
        }
        let unfixed = !matches!(sig.width, Width::Fixed(_));
        Some((raw(ty), OperandPost::Float(operand, unfixed, own)))
    }

    fn plan_builtin(
        &mut self,
        node: NodeId,
        builtin: Builtin,
        args: &[NodeId],
        (own, flows): (EvalTy, bool),
        build: &mut Build,
    ) -> Result<Planned, Internal> {
        let set = build.set();
        let mut list = Vec::new();
        if builtin == Builtin::Rand {
            list.push(Task::Set(
                args.iter().map(|arg| Req::Raw(*arg, EvalTy::I64)).collect(),
                set,
            ));
            list.push(Task::Emit(Instr::Rand(Rc::from(args), Lay::Set(set))));
            return Ok(Planned::Tasks(list));
        }
        let mut reqs = Vec::with_capacity(args.len());
        let mut posts = Vec::with_capacity(args.len());
        for arg in args {
            if is_float_builtin(builtin) {
                let Some((req, post)) = self.float_plan(*arg, (own, flows), false) else {
                    return Ok(Planned::Needs(vec![Query::Sig(*arg)]));
                };
                reqs.push(req);
                posts.push(post);
            } else {
                reqs.push(Req::Typed(*arg, own, flows));
                posts.push(OperandPost::Keep);
            }
        }
        list.push(Task::Set(reqs, set));
        list.push(Task::Emit(Instr::Builtin(
            node,
            builtin,
            own,
            Rc::new(posts),
            Lay::Set(set),
        )));
        Ok(Planned::Tasks(list))
    }

    fn leaf_plan(&self, arg: NodeId, own: EvalTy) -> Result<LeafPlan, Internal> {
        let Some(sig) = self.compiled_sig(arg) else {
            return Ok(Err(vec![Query::Sig(arg)]));
        };
        if sig.kind == Kind::Bare {
            return Ok(Ok(match self.carried_enum_of(arg)? {
                Some(id) => (
                    LeafFact::Value(false, EvalTy::Enum(id), None),
                    Some(Req::Raw(arg, EvalTy::Enum(id))),
                ),
                None => (LeafFact::Unplaced(arg, own), None),
            }));
        }
        let flexible = sig.width == Width::Flexible;
        let ty = if flexible && sig.kind == Kind::Int {
            EvalTy::Exact
        } else {
            own_type(&sig, own)
        };
        let chosen = sig.kind == Kind::Node && sig.frame && !sig.reactive && !sig.is_error();
        let req = if chosen {
            Req::StoredTyped(arg, ty, false)
        } else {
            Req::Typed(arg, ty, false)
        };
        Ok(Ok((LeafFact::Value(flexible, ty, sig.family), Some(req))))
    }

    fn plan_call(
        &mut self,
        node: NodeId,
        function: crate::semantics::ir::FnId,
        args: &[Option<Val>],
        own: EvalTy,
        build: &mut Build,
    ) -> Result<Planned, Internal> {
        let mut facts = Vec::with_capacity(args.len());
        let mut reqs = Vec::new();
        let mut needs = Vec::new();
        for arg in args {
            match arg {
                None => facts.push(ArgFact::Default),
                Some(Val::Error) => facts.push(ArgFact::Error),
                Some(Val::Scalar(arg)) => match self.leaf_plan(*arg, own)? {
                    Ok((fact, req)) => {
                        facts.push(ArgFact::Leaf(fact));
                        reqs.extend(req);
                    }
                    Err(missing) => needs.extend(missing),
                },
                Some(Val::Array(array)) => {
                    let mut elements = Vec::with_capacity(array.elements.len());
                    for element in &array.elements {
                        match self.leaf_plan(*element, own)? {
                            Ok((fact, req)) => {
                                elements.push(fact);
                                reqs.extend(req);
                            }
                            Err(missing) => needs.extend(missing),
                        }
                    }
                    facts.push(ArgFact::Elements(
                        Rc::from(elements),
                        Rc::from(array.shape.lengths()),
                    ));
                }
            }
        }
        if !needs.is_empty() {
            return Ok(Planned::Needs(needs));
        }
        let count = u32::try_from(reqs.len()).map_err(|_| Internal::missed("argument"))?;
        let set = build.set();
        Ok(Planned::Tasks(vec![
            Task::Set(reqs, set),
            Task::Emit(Instr::CallWith(Box::new(CallInstr {
                node,
                own,
                function,
                facts: Rc::from(facts),
                count,
                lay: Lay::Set(set),
                seen: std::cell::RefCell::new(None),
            }))),
        ]))
    }

    fn plan_for(
        &mut self,
        node: NodeId,
        for_node: &crate::semantics::analysis::for_node::ForNode,
        (own, flows): (EvalTy, bool),
        build: &mut Build,
    ) -> Result<Planned, Internal> {
        let end = build.label();
        let items = build.set();
        let mut list = vec![Task::Emit(Instr::LoopStart(node, own, end))];
        match for_node.iterable {
            ForIterable::Range(start, stop) => {
                list.push(Task::Set(
                    vec![
                        Req::Raw(start, EvalTy::Exact),
                        Req::Raw(stop, EvalTy::Exact),
                    ],
                    items,
                ));
                list.push(Task::Emit(Instr::ForCheck(true, Lay::Set(items), end)));
            }
            ForIterable::Values(values) => {
                let sig = match self.sigs_of(&[values]) {
                    Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                let element = for_node.element.map(|slot| self.slot(slot)).transpose()?;
                let annotated = element.and_then(|slot| match (slot.annotated, slot.sig.width) {
                    (true, Width::Fixed(scalar)) => Some(eval_ty_of(Some(scalar))),
                    _ => None,
                });
                let ty = if let Some(annotated) = annotated {
                    annotated
                } else if sig.kind == Kind::Int && sig.width == Width::Flexible {
                    EvalTy::Exact
                } else {
                    own_type(&sig, EvalTy::Default)
                };
                list.push(Task::Set(vec![Req::Raw(values, ty)], items));
                if let Some(annotated) = annotated {
                    list.push(Task::Emit(Instr::Fit(values, annotated)));
                }
                list.push(Task::Emit(Instr::ForCheck(false, Lay::Plain, end)));
            }
        }
        let start = build.set();
        list.push(Task::Set(
            vec![Req::Typed(for_node.start, own, flows)],
            start,
        ));
        list.push(Task::Emit(Instr::ForRun(
            node,
            Rc::new(for_node.clone()),
            own,
            Lay::Set(items),
        )));
        list.push(Task::Mark(end));
        Ok(Planned::Tasks(list))
    }

    fn plan_native(
        &mut self,
        node: NodeId,
        native: symphonyscript_core::NativeId,
        args: &[NodeId],
        build: &mut Build,
    ) -> Result<Planned, Internal> {
        let decl = self
            .vocabulary
            .native(native)
            .ok_or(Internal::missed("native function"))?;
        let params = &decl.signature.params;
        let mut fits = true;
        let mut checks = Vec::new();
        for (arg, param) in args.iter().zip(params) {
            let scalar = native_ty(param).scalar;
            let array = self
                .native_argument(*arg, None)?
                .is_some_and(|leaves| leaves != [*arg]);
            if array {
                let sig = self.known_sig(*arg)?;
                if sig.is_error() {
                    fits = false;
                    continue;
                }
                if let Some((code, message)) = self.incompatibility(*arg, &sig, scalar, false)? {
                    checks.push(Task::Emit(Instr::Draw(code, *arg, Rc::from(message))));
                    fits = false;
                    continue;
                }
            }
            let Some(leaves) = self.native_argument(*arg, None)? else {
                fits = false;
                continue;
            };
            for leaf in leaves {
                let sig = match self.sigs_of(&[leaf]) {
                    Ok(sigs) => sigs.first().copied().unwrap_or(Sig::ERROR),
                    Err(needs) => return Ok(Planned::Needs(needs)),
                };
                if sig.is_error() {
                    fits = false;
                    continue;
                }
                if let Some((code, message)) = self.incompatibility(leaf, &sig, scalar, false)? {
                    checks.push(Task::Emit(Instr::Draw(code, leaf, Rc::from(message))));
                    fits = false;
                    continue;
                }
                let ty = of_scalar(scalar);
                if sig.kind == Kind::Int && sig.width == Width::Flexible && is_int_ty(ty) {
                    let set = build.set();
                    checks.push(Task::Set(vec![Req::Raw(leaf, ty)], set));
                    checks.push(Task::Emit(Instr::FitAnd(leaf, ty)));
                }
            }
        }
        let end = build.label();
        let mut list = vec![Task::Emit(Instr::Push(CVal::Bool(fits)))];
        list.extend(checks);
        list.push(Task::Emit(Instr::Gate(end)));
        let set = build.set();
        let reqs: Vec<Req> = args
            .iter()
            .zip(params)
            .map(|(arg, param)| Req::Raw(*arg, of_scalar(native_ty(param).scalar)))
            .collect();
        let count = u32::try_from(reqs.len()).map_err(|_| Internal::missed("native argument"))?;
        list.push(Task::Set(reqs, set));
        list.push(Task::Emit(Instr::Native(
            node,
            native,
            count,
            Lay::Set(set),
        )));
        list.push(Task::Mark(end));
        Ok(Planned::Tasks(list))
    }
}

fn pure(list: &[Task]) -> bool {
    match list {
        [Task::Emit(instr)] => match instr {
            Instr::Arg(_) | Instr::Slot(_) | Instr::Push(_) => true,
            Instr::Held(_)
            | Instr::Child(..)
            | Instr::Fetch(_)
            | Instr::Narrow(_)
            | Instr::Fault(..)
            | Instr::Draw(..)
            | Instr::Convert(_)
            | Instr::Fit(..)
            | Instr::Shape(..)
            | Instr::Scaled(..)
            | Instr::ScaledInto(..)
            | Instr::BeforeUnit(..)
            | Instr::NarrowFloat(_)
            | Instr::Unless(..)
            | Instr::Jump(_)
            | Instr::BailIfError(_)
            | Instr::Pick(..)
            | Instr::MatchLabel(_)
            | Instr::Drop
            | Instr::IfGlobal(_)
            | Instr::Element(..)
            | Instr::Array(..)
            | Instr::Not
            | Instr::Neg(..)
            | Instr::Logic(..)
            | Instr::Arith(..)
            | Instr::Compare(..)
            | Instr::FlexPair(_)
            | Instr::StrAdd(..)
            | Instr::IntDivFloat(..)
            | Instr::Div(..)
            | Instr::FromEnd(..)
            | Instr::DeferredCheck(..)
            | Instr::DeferredSum(..)
            | Instr::ConvertTo(_)
            | Instr::Join(..)
            | Instr::Slice(..)
            | Instr::Length(_)
            | Instr::Bytes(..)
            | Instr::Capacity(..)
            | Instr::Rand(..)
            | Instr::Builtin(..)
            | Instr::Concat(..)
            | Instr::FitAnd(..)
            | Instr::Gate(_)
            | Instr::Native(..)
            | Instr::CallWith(..)
            | Instr::LoopStart(..)
            | Instr::ForCheck(..)
            | Instr::ForRun(..)
            | Instr::FrameRead(..)
            | Instr::ToRead(..) => false,
        },
        _ => false,
    }
}

fn operand_fit(node: NodeId, sig: &Sig) -> Option<NodeId> {
    (sig.width == Width::Flexible && sig.is_numeric()).then_some(node)
}

fn fit_ty(ty: EvalTy) -> EvalTy {
    match ty {
        EvalTy::Exact => EvalTy::I64,
        EvalTy::Default
        | EvalTy::U8
        | EvalTy::I32
        | EvalTy::I64
        | EvalTy::F32
        | EvalTy::F64
        | EvalTy::DefaultF32
        | EvalTy::DefaultF64
        | EvalTy::Bool
        | EvalTy::Enum(_)
        | EvalTy::Str
        | EvalTy::Node => ty,
    }
}

fn converts(kind: Kind, ty: EvalTy) -> bool {
    match kind {
        Kind::Int => is_float_ty(ty),
        Kind::Float => matches!(
            ty,
            EvalTy::F64
                | EvalTy::DefaultF64
                | EvalTy::U8
                | EvalTy::I32
                | EvalTy::I64
                | EvalTy::Exact
        ),
        Kind::Bool | Kind::Str | Kind::Node | Kind::Enum(_) | Kind::Bare => false,
        Kind::Any | Kind::Error => true,
    }
}
