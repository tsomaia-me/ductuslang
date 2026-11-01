use super::checks::Checks;
use super::ctx::Ctx;
use super::frame::Frame;
use super::frame_spare::FrameSpare;
use super::loop_found::LoopFound;
use super::loop_items::LoopItems;
use super::loop_run::LoopRun;
use super::machine::Machine;
use super::machine_key::MachineKey;
use super::mode::Mode;
use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::for_node::ForNode;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::reachable::Reachable;
use crate::semantics::analysis::sink::Sink;
use crate::semantics::analysis::slot_owner::SlotOwner;

impl Engine<'_> {
    pub(super) fn loop_value(
        &self,
        node: NodeId,
        own: EvalTy,
        ctx: Ctx,
    ) -> Result<LoopFound, Internal> {
        let Ctx::Frame(frame) = ctx else {
            return Ok(LoopFound::Missing);
        };
        let record = self.frame(frame)?;
        if let Some(slot) = self.frame_slot(record.owner, node)?
            && let Some(value) = record.loops.get(slot, own)
        {
            return Ok(LoopFound::Value(value.clone()));
        }
        Ok(LoopFound::Missing)
    }

    pub(super) fn loop_start(
        &mut self,
        node: NodeId,
        own: EvalTy,
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<LoopFound, Internal> {
        if let Ctx::Frame(_) = ctx {
            return self.loop_value(node, own, ctx);
        }
        let request = self.sink;
        self.link(&request, Reachable::Loop(node, own));
        let key = MachineKey::Loop(node, own);
        let Some(machine) = self.machines.remove(&key) else {
            return Ok(LoopFound::Missing);
        };
        self.active = Some(machine);
        Ok(match self.drive(key, node, need)? {
            Some(value) => LoopFound::Value(value),
            None => LoopFound::Pending,
        })
    }

    pub(super) fn for_run(
        &mut self,
        node: NodeId,
        for_node: &ForNode,
        own: EvalTy,
        (items, start): (LoopItems, CVal),
        ctx: Ctx,
        need: &mut Vec<Query>,
    ) -> Result<Option<CVal>, Internal> {
        let key = MachineKey::Loop(node, own);
        if start.is_error() || items.count() == 0 {
            return Ok(Some(start));
        }
        if !self.counted_iteration(node)? {
            return Ok(Some(CVal::Error));
        }
        let (args, feeds, depth, mode, sink, untaken, owner) = match ctx {
            Ctx::Frame(frame) => {
                let parent = self.frame(frame)?;
                (
                    parent.args.clone(),
                    parent.feeds.clone(),
                    parent.depth,
                    self.mode_of(frame)?,
                    self.frame_sink(frame)?,
                    (parent.untaken_depth, parent.untaken_start),
                    parent.owner,
                )
            }
            Ctx::Global => (
                Vec::new(),
                self.memo.call_feeds.get(&node).cloned().unwrap_or_default(),
                0,
                Mode::Taken,
                Sink::Into(Reachable::Loop(node, own)),
                (0, 0),
                SlotOwner::Root(node),
            ),
        };
        let spare = match ctx {
            Ctx::Frame(_) => self.machine_mut()?.spares.pop().unwrap_or_default(),
            Ctx::Global => FrameSpare::default(),
        };
        let first = items.item(0);
        let record = Frame {
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
            owner,
            values: spare.values,
            loops: spare.loops,
            held: None,
            depth,
            faults: spare.faults,
            templates: spare.templates,
            checks: Checks::Done,
            mode,
            sink,
            id: None,
            opened: spare.opened,
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
                    walked: FastSet::default(),
                    spares: Vec::new(),
                    stack: Vec::new(),
                    resumes: Vec::new(),
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
            record.values.clear();
            record.loops.clear();
            record.held = None;
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
            let owner = parent.owner;
            let (call, ty) = (done.call, done.ty);
            machine.spares.push(FrameSpare::kept(done));
            let slot = self.numbered_slot(owner, call)?;
            self.machine_mut()?
                .frames
                .last_mut()
                .ok_or(Internal::missed("evaluation frame"))?
                .loops
                .insert(slot, ty, value);
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
