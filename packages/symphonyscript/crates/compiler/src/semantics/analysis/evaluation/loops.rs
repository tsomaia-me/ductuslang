use super::checks::Checks;
use super::ctx::Ctx;
use super::frame::Frame;
use super::loop_items::LoopItems;
use super::loop_run::LoopRun;
use super::machine::Machine;
use super::machine_key::MachineKey;
use super::mode::Mode;
use super::own_type::own_type;
use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::eval_ty_of::eval_ty_of;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::for_iterable::ForIterable;
use crate::semantics::analysis::for_node::ForNode;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::reachable::Reachable;
use crate::semantics::analysis::sink::Sink;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::width::Width;

impl Engine<'_> {
    pub(super) fn for_value(
        &mut self,
        node: NodeId,
        for_node: &ForNode,
        (own, partner): (EvalTy, Option<EvalTy>),
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let key = MachineKey::Loop(node, own);
        match ctx {
            Ctx::Frame(frame) => {
                if let Some(value) = self.frame(frame)?.loops.get(&(node, own)) {
                    return Ok(Some(value.clone()));
                }
            }
            Ctx::Global => {
                let request = self.sink;
                self.link(&request, Reachable::Loop(node, own));
                if let Some(machine) = self.machines.remove(&key) {
                    self.active = Some(machine);
                    return self.drive(key, node, need);
                }
            }
        }
        let items = match for_node.iterable {
            ForIterable::Range(start, end) => {
                let start = self.child(start, EvalTy::Exact, ctx, need)?;
                let end = self.child(end, EvalTy::Exact, ctx, need)?;
                match (some!(start), some!(end)) {
                    (CVal::Int(start), CVal::Int(end)) => LoopItems::Range {
                        start,
                        count: usize::try_from(end.saturating_sub(start).max(0))
                            .unwrap_or(usize::MAX),
                    },
                    _ => return Ok(Some(CVal::Error)),
                }
            }
            ForIterable::Values(values) => {
                let sig = some!(self.sig(values, need));
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
                let value = some!(self.child(values, ty, ctx, need)?);
                let value = match annotated {
                    Some(annotated) => self.fit(values, value, annotated)?,
                    None => value,
                };
                match value {
                    CVal::Array(array) => LoopItems::Array(array),
                    CVal::Int(_)
                    | CVal::F32(_)
                    | CVal::F64(_)
                    | CVal::Bool(_)
                    | CVal::Enum(..)
                    | CVal::Str(_)
                    | CVal::Node(_)
                    | CVal::Error => return Ok(Some(CVal::Error)),
                }
            }
        };
        let start = some!(self.typed_child(for_node.start, (own, partner), ctx, need)?);
        if start.is_error() || items.count() == 0 {
            return Ok(Some(start));
        }
        if !self.counted_iteration(node)? {
            return Ok(Some(CVal::Error));
        }
        let (args, function, feeds, depth, mode, sink, untaken) = match ctx {
            Ctx::Frame(frame) => {
                let parent = self.frame(frame)?;
                (
                    parent.args.clone(),
                    parent.function,
                    parent.feeds.clone(),
                    parent.depth,
                    self.mode_of(frame)?,
                    self.frame_sink(frame)?,
                    (parent.untaken_depth, parent.untaken_start),
                )
            }
            Ctx::Global => (
                Vec::new(),
                None,
                self.memo.call_feeds.get(&node).cloned().unwrap_or_default(),
                0,
                Mode::Taken,
                Sink::Into(Reachable::Loop(node, own)),
                (0, 0),
            ),
        };
        let first = items.item(0);
        let record = Frame {
            key: None,
            function,
            run: Some(LoopRun {
                items,
                next: 0,
                index: for_node.index,
                element: for_node.element,
                accumulator: for_node.accumulator,
                values: [CVal::Int(0), first, start],
            }),
            args,
            call: node,
            root: for_node.body,
            ty: own,
            expected: own,
            feeds,
            memo: FastMap::default(),
            loops: FastMap::default(),
            depth,
            faults: FastSet::default(),
            templates: FastSet::default(),
            checks: Checks::Done,
            mode,
            sink,
            id: None,
            opened: FastSet::default(),
            untaken_depth: untaken.0,
            untaken_start: untaken.1,
            height: 1,
            callees: Vec::new(),
        };
        match ctx {
            Ctx::Frame(_) => {
                let machine = self.machine_mut()?;
                let index = machine.frames.len();
                machine.frames.push(record);
                machine.work.push((index, for_node.body, own));
                self.frame_pushed = true;
                Ok(None)
            }
            Ctx::Global => {
                self.active = Some(Machine {
                    frames: vec![record],
                    work: vec![(0, for_node.body, own)],
                    open_keys: FastSet::default(),
                    rerun: FastSet::default(),
                    too_deep: false,
                    passed: None,
                    deep_warned: false,
                });
                self.drive(key, node, need)
            }
        }
    }

    pub(super) fn loop_step(
        &mut self,
        frame: usize,
        value: CVal,
    ) -> Result<Option<CVal>, Internal> {
        let (call, next, count) = {
            let record = self.frame(frame)?;
            let run = record.run.as_ref().ok_or(Internal::missed("loop run"))?;
            (record.call, run.next + 1, run.items.count())
        };
        let limited = if value.is_error() || next >= count {
            Some(false)
        } else if !self.counted_iteration(call)? {
            Some(true)
        } else {
            None
        };
        let machine = self.machine_mut()?;
        let Some(limited) = limited else {
            let record = machine
                .frames
                .get_mut(frame)
                .ok_or(Internal::missed("evaluation frame"))?;
            let run = record.run.as_mut().ok_or(Internal::missed("loop run"))?;
            run.next = next;
            let position = i128::try_from(next).map_err(|_| Internal::missed("loop position"))?;
            let element = run.items.item(next);
            run.values = [CVal::Int(position), element, value];
            record.memo.clear();
            record.loops.clear();
            let (root, ty) = (record.root, record.ty);
            machine.work.push((frame, root, ty));
            return Ok(None);
        };
        let value = if limited { CVal::Error } else { value };
        let done = machine
            .frames
            .pop()
            .ok_or(Internal::missed("evaluation frame"))?;
        if let Some(parent) = machine.frames.last_mut() {
            parent.faults.extend(done.faults.iter().copied());
            parent.templates.extend(done.templates.iter().copied());
            parent.opened.extend(done.opened.iter().copied());
            parent.height = parent.height.max(done.height);
            parent.loops.insert((done.call, done.ty), value);
            return Ok(None);
        }
        self.active = None;
        let reach: (Vec<u32>, Vec<ScopeId>) = (
            done.faults.into_iter().collect(),
            done.templates.into_iter().collect(),
        );
        self.reach(reach, call, Ctx::Global)?;
        Ok(Some(value))
    }

    pub(super) fn slot_value(
        &self,
        frame: usize,
        slot: crate::semantics::analysis::ids::SlotId,
    ) -> Result<CVal, Internal> {
        let machine = self
            .active
            .as_ref()
            .ok_or(Internal::missed("evaluation machine"))?;
        let frames = machine
            .frames
            .get(..=frame)
            .ok_or(Internal::missed("evaluation frame"))?;
        frames
            .iter()
            .rev()
            .find_map(|record| record.run.as_ref().and_then(|run| run.value(slot)))
            .cloned()
            .ok_or(Internal::missed("loop slot"))
    }

    fn counted_iteration(&mut self, node: NodeId) -> Result<bool, Internal> {
        let (loc, scope) = {
            let record = self.node(node)?;
            (record.loc, record.scope)
        };
        self.count_step(loc, scope)
    }
}
