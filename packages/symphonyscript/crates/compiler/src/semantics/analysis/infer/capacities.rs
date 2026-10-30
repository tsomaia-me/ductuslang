use std::collections::VecDeque;

use symphonyscript_core::{Builtin, NativeType};

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::answers::CellRead;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::capacity::{BoundForm, Extreme, Piecewise, Ratio, slice_transform};
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::for_iterable::ForIterable;
use crate::semantics::analysis::for_node::ForNode;
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId, SlotId};
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::resolve::LayoutFound;
use crate::semantics::collection::strongly_connected;
use crate::semantics::ir::StateId;

const LIMIT: i128 = 1 << 31;

const STEPS: u64 = 1 << 20;

type Env = FastMap<BindingId, Piecewise>;

enum Parts {
    Leaf(Extent),
    Operands(Vec<NodeId>, Option<Extent>),
}

#[derive(Clone)]
enum Extent {
    Fixed(i128),
    Variable(Piecewise),
}

impl Extent {
    fn capacity(&self) -> Piecewise {
        match self {
            Extent::Fixed(length) => Piecewise::constant(*length),
            Extent::Variable(capacity) => capacity.clone(),
        }
    }

    fn join(&self, other: &Extent) -> Extent {
        match (self, other) {
            (Extent::Fixed(a), Extent::Fixed(b)) => Extent::Fixed(a.saturating_add(*b)),
            _ => Extent::Variable(self.capacity().add(&other.capacity())),
        }
    }

    fn choice(&self, other: &Extent) -> Extent {
        match (self, other) {
            (Extent::Fixed(a), Extent::Fixed(b)) if a == b => Extent::Fixed(*a),
            _ => Extent::Variable(self.capacity().max(&other.capacity())),
        }
    }
}

fn fixed_slice(lower: BoundForm, upper: BoundForm, base: &Extent) -> Option<i128> {
    let Extent::Fixed(length) = base else {
        return None;
    };
    let position = |form: BoundForm, absent: i128| match form {
        BoundForm::Absent => Some(absent),
        BoundForm::Reactive(_) => None,
        BoundForm::Start(value) => Some(value.clamp(0, *length)),
        BoundForm::End(offset) => Some(length.saturating_sub(offset).clamp(0, *length)),
    };
    let start = position(lower, 0)?;
    let end = position(upper, *length)?;
    Some(end.max(start) - start)
}

fn outside_i64(form: BoundForm) -> bool {
    match form {
        BoundForm::Start(value) | BoundForm::End(value) => i64::try_from(value).is_err(),
        BoundForm::Absent | BoundForm::Reactive(_) => false,
    }
}

struct Streak {
    period: usize,
    direction: Vec<i128>,
    phases: usize,
    run: Option<Ratio>,
}

fn repeating(history: &VecDeque<Vec<i128>>, longest: usize) -> Option<(usize, Vec<i128>)> {
    let newest = history.len().checked_sub(1)?;
    (1..=longest).find_map(|period| {
        let middle = history.get(newest.checked_sub(period)?)?;
        let oldest = history.get(newest.checked_sub(2 * period)?)?;
        let last = history.get(newest)?;
        let direction: Vec<i128> = last.iter().zip(middle).map(|(a, b)| a - b).collect();
        let repeats = middle
            .iter()
            .zip(oldest)
            .map(|(a, b)| a - b)
            .eq(direction.iter().copied());
        let growing =
            direction.iter().all(|slope| *slope >= 0) && direction.iter().any(|slope| *slope > 0);
        (repeats && growing).then_some((period, direction))
    })
}

enum Solved {
    Capacities(Vec<i128>),
    Limit(Vec<i128>),
    Unbounded(Vec<bool>),
}

const CEILING: i128 = 1 << 120;

fn joined(total: i128) -> String {
    format!(
        "this join holds up to {total} elements, and an array's length and capacity are below 2³¹"
    )
}

fn reaches_limit(extent: Option<&Extent>) -> bool {
    match extent {
        Some(Extent::Fixed(length)) => *length >= LIMIT,
        Some(Extent::Variable(capacity)) => capacity.start().floor() >= LIMIT,
        None => false,
    }
}

impl Engine<'_> {
    pub(super) fn capacities(&mut self) -> Result<(), Internal> {
        let mut members: Vec<(StateId, BindingId)> = Vec::new();
        for index in 0..self.hir.states.len() {
            crate::semantics::counted_walk::CountedWalk::CapacitiesCapacities.step();
            let state = StateId(u32::try_from(index).unwrap_or(u32::MAX));
            let Some(BoundVal::Scalar(Bound::Cell(binding))) =
                self.memo.states.get(&state).cloned()
            else {
                continue;
            };
            let variable = self
                .memo
                .binding_layouts
                .get(&binding)
                .is_some_and(|layout| layout.variable);
            if variable && !self.memo.binding_capacities.contains_key(&binding) {
                members.push((state, binding));
            }
        }
        let positions: FastMap<BindingId, usize> = members
            .iter()
            .enumerate()
            .map(|(position, (_, binding))| (*binding, position))
            .collect();
        let mut edges = Vec::with_capacity(members.len());
        for (_, binding) in &members {
            let init = self.binding(*binding)?.init;
            let mut targets: Vec<usize> = self
                .state_dependencies(init)?
                .into_iter()
                .filter_map(|dependency| positions.get(&dependency).copied())
                .collect();
            targets.sort_unstable();
            targets.dedup();
            edges.push(targets);
        }
        let mut covered: FastSet<NodeId> = FastSet::default();
        for component in strongly_connected(&edges)? {
            let group: Vec<(StateId, BindingId)> = component
                .iter()
                .map(|position| {
                    members
                        .get(*position)
                        .copied()
                        .ok_or(Internal::missed("state"))
                })
                .collect::<Result<_, _>>()?;
            match self.solve(&group)? {
                Solved::Capacities(capacities) => {
                    for ((_, binding), capacity) in group.iter().zip(capacities) {
                        let capacity = u32::try_from(capacity).unwrap_or(u32::MAX);
                        self.memo.binding_capacities.insert(*binding, capacity);
                    }
                }
                Solved::Limit(reached) => {
                    self.limited(&group, &reached, &mut covered)?;
                    for (_, binding) in &group {
                        self.memo.binding_capacities.insert(*binding, 0);
                    }
                }
                Solved::Unbounded(growing) => {
                    for (_, binding) in &group {
                        let init = self.binding(*binding)?.init;
                        self.cover(init, &mut covered)?;
                    }
                    let first = group
                        .iter()
                        .zip(&growing)
                        .filter(|(_, grows)| **grows)
                        .map(|((state, _), _)| *state)
                        .min();
                    if let Some(state) = first {
                        self.unbounded(state)?;
                    }
                    for (_, binding) in &group {
                        self.memo.binding_capacities.insert(*binding, 0);
                    }
                }
            }
        }
        self.length_limits(&covered)
    }

    fn limited(
        &mut self,
        group: &[(StateId, BindingId)],
        reached: &[i128],
        covered: &mut FastSet<NodeId>,
    ) -> Result<(), Internal> {
        let env: Env = group
            .iter()
            .zip(reached)
            .map(|((_, binding), value)| (*binding, Piecewise::constant(*value)))
            .collect();
        let mut done: FastMap<NodeId, Extent> = FastMap::default();
        for (_, binding) in group {
            let init = self.binding(*binding)?.init;
            self.extent_into(init, &env, &mut done)?;
            self.cover(init, covered)?;
        }
        let mut joins: Vec<NodeId> = Vec::new();
        for (node, extent) in &done {
            if !reaches_limit(Some(extent)) {
                continue;
            }
            if let Op::Join(lhs, rhs) = self.node(*node)?.op
                && !reaches_limit(done.get(&lhs))
                && !reaches_limit(done.get(&rhs))
            {
                joins.push(*node);
            }
        }
        joins.sort_unstable();
        for join in &joins {
            let total = done
                .get(join)
                .map_or(LIMIT, |extent| extent.capacity().start().floor());
            self.error_at_node(Code::LengthLimit, *join, joined(total))?;
        }
        if !joins.is_empty() {
            return Ok(());
        }
        let first = group
            .iter()
            .zip(reached)
            .filter(|(_, value)| **value >= LIMIT)
            .min_by_key(|((state, _), _)| *state);
        if let Some(((state, binding), value)) = first {
            let init = self.binding(*binding)?.init;
            let name = self
                .hir
                .state(*state)?
                .name
                .as_ref()
                .map_or(String::new(), |name| name.text.clone());
            self.error_at_node(
                Code::LengthLimit,
                init,
                format!(
                    "`{name}` keeps up to {value} elements, and an array's length and capacity are below 2³¹"
                ),
            )?;
        }
        Ok(())
    }

    fn cover(&self, init: NodeId, covered: &mut FastSet<NodeId>) -> Result<(), Internal> {
        let mut stack = vec![init];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::CapacitiesCover.step();
            if !covered.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::Read(_) => {}
                Op::History { fallback, .. } => stack.extend(fallback.iter().copied()),
                op => stack.extend(op.operands()),
            }
        }
        Ok(())
    }

    fn length_limits(&mut self, covered: &FastSet<NodeId>) -> Result<(), Internal> {
        let env = Env::default();
        let mut done: FastMap<NodeId, Extent> = FastMap::default();
        for index in 0..self.nodes.len() {
            crate::semantics::counted_walk::CountedWalk::CapacitiesLengthLimits.step();
            let node = NodeId(u32::try_from(index).map_err(|_| Internal::missed("node"))?);
            if let Op::For(for_node) = &self.node(node)?.op
                && let ForIterable::Range(low, high) = for_node.iterable
            {
                self.range_bounds(node, (low, high))?;
                continue;
            }
            let Op::Join(lhs, rhs) = self.node(node)?.op else {
                continue;
            };
            if covered.contains(&node) {
                continue;
            }
            let variable = matches!(
                self.known_layout(node)?.shaped,
                Ok(Some(layout)) if layout.variable
            );
            if !variable {
                continue;
            }
            self.extent_into(node, &env, &mut done)?;
            if reaches_limit(done.get(&node))
                && !reaches_limit(done.get(&lhs))
                && !reaches_limit(done.get(&rhs))
            {
                let total = done
                    .get(&node)
                    .map_or(LIMIT, |extent| extent.capacity().start().floor());
                self.error_at_node(Code::LengthLimit, node, joined(total))?;
            }
        }
        Ok(())
    }

    fn unbounded(&mut self, state: StateId) -> Result<(), Internal> {
        let decl = self.hir.state(state)?;
        let name = decl
            .name
            .as_ref()
            .map_or(String::new(), |name| name.text.clone());
        let loc = decl.name.as_ref().map_or(decl.loc, |name| name.loc);
        let scope = self.source_global;
        self.error(
            Code::UnboundedState,
            loc,
            scope,
            format!(
                "`{name}` grows at every step, so no capacity holds it; a slice such as `[^8..]` around its initializer keeps the newest eight elements"
            ),
        )
    }

    fn solve(&mut self, group: &[(StateId, BindingId)]) -> Result<Solved, Internal> {
        let inits: Vec<NodeId> = group
            .iter()
            .map(|(_, binding)| self.binding(*binding).map(|record| record.init))
            .collect::<Result<_, _>>()?;
        let members: Vec<BindingId> = group.iter().map(|(_, binding)| *binding).collect();
        let longest = members.len() + 1;
        let mut history: VecDeque<Vec<i128>> = VecDeque::from([vec![0i128; members.len()]]);
        let mut streak: Option<Streak> = None;
        for _ in 0..STEPS {
            let current = history
                .back()
                .cloned()
                .ok_or(Internal::missed("capacity"))?;
            let next = self.capacity_step(&members, &inits, &current)?;
            if next.iter().any(|value| *value >= CEILING) {
                return Ok(Solved::Unbounded(
                    next.iter().map(|value| *value >= LIMIT).collect(),
                ));
            }
            if next == current {
                return Ok(if current.iter().any(|value| *value >= LIMIT) {
                    Solved::Limit(current)
                } else {
                    Solved::Capacities(current)
                });
            }
            history.push_back(next.clone());
            if history.len() > 2 * longest + 1 {
                history.pop_front();
            }
            let Some((period, direction)) = repeating(&history, longest) else {
                streak = None;
                continue;
            };
            let run = self.phase_run(&members, &inits, (&current, &next), &direction)?;
            let found = match streak.take() {
                Some(mut found) if found.period == period && found.direction == direction => {
                    found.phases += 1;
                    found.run = match (found.run, run) {
                        (Some(left), Some(right)) => Some(left.min(right)),
                        (left, right) => left.or(right),
                    };
                    found
                }
                Some(_) | None => Streak {
                    period,
                    direction,
                    phases: 1,
                    run,
                },
            };
            if found.phases >= found.period {
                match found.run {
                    None => {
                        return Ok(Solved::Unbounded(
                            found.direction.iter().map(|slope| *slope > 0).collect(),
                        ));
                    }
                    Some(run) if run.floor() >= 2 => {
                        let jumps = run.floor();
                        let jumped = next
                            .iter()
                            .zip(&found.direction)
                            .map(|(start, slope)| start.saturating_add(slope.saturating_mul(jumps)))
                            .collect();
                        history = VecDeque::from([jumped]);
                        continue;
                    }
                    Some(_) => continue,
                }
            }
            streak = Some(found);
        }
        Ok(Solved::Unbounded(vec![true; members.len()]))
    }

    fn capacity_step(
        &mut self,
        members: &[BindingId],
        inits: &[NodeId],
        current: &[i128],
    ) -> Result<Vec<i128>, Internal> {
        let env: Env = members
            .iter()
            .zip(current)
            .map(|(binding, value)| (*binding, Piecewise::constant(*value)))
            .collect();
        inits
            .iter()
            .map(|init| Ok(self.capacity_with(*init, &env)?.start().floor()))
            .collect()
    }

    fn phase_run(
        &mut self,
        members: &[BindingId],
        inits: &[NodeId],
        (current, next): (&[i128], &[i128]),
        direction: &[i128],
    ) -> Result<Option<Ratio>, Internal> {
        let env: Env = members
            .iter()
            .zip(current)
            .zip(direction)
            .map(|((binding, start), slope)| (*binding, Piecewise::affine(*start, *slope)))
            .collect();
        let mut run: Option<Ratio> = None;
        for ((init, after), slope) in inits.iter().zip(next).zip(direction) {
            let function = self.capacity_with(*init, &env)?;
            let shifted = function.add(&Piecewise::affine(*after, *slope).neg());
            if let Some(length) = shifted.zero_run() {
                run = Some(run.map_or(length, |shortest| shortest.min(length)));
            }
        }
        Ok(run)
    }

    pub(super) fn state_dependencies(&mut self, init: NodeId) -> Result<Vec<BindingId>, Internal> {
        let mut found = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = vec![init];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::CapacitiesStateDependencies.step();
            if !seen.insert(node) {
                continue;
            }
            match self.cell_read(node)? {
                CellRead::Binding(binding) => {
                    if self.is_state(binding)? {
                        found.push(binding);
                    } else {
                        stack.push(self.binding(binding)?.init);
                    }
                }
                CellRead::History { cell, fallback } => {
                    if let Some(binding) = self.history_cell_binding(cell)? {
                        if self.is_state(binding)? {
                            found.push(binding);
                        } else {
                            stack.push(self.binding(binding)?.init);
                        }
                    }
                    stack.extend(fallback);
                }
                CellRead::Named(inner) => stack.push(inner),
                CellRead::Operands(operands) => stack.extend(operands),
            }
        }
        Ok(found)
    }

    fn is_state(&self, binding: BindingId) -> Result<bool, Internal> {
        Ok(matches!(
            self.binding(binding)?.kind,
            crate::semantics::analysis::binding_kind::BindingKind::State { .. }
        ))
    }

    fn history_cell_binding(&self, cell: HistoryCell) -> Result<Option<BindingId>, Internal> {
        let bound = match cell {
            HistoryCell::Input(input) => self.memo.inputs.get(&input),
            HistoryCell::Let(id) => self
                .memo
                .lets
                .get(&(self.global_of_let(id), LetRef::Top(id))),
            HistoryCell::State(state) => self.memo.states.get(&state),
        };
        Ok(match bound {
            Some(BoundVal::Scalar(Bound::Cell(binding))) => Some(*binding),
            Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Array(_) | BoundVal::Error)
            | None => None,
        })
    }

    pub(in crate::semantics::analysis) fn known_layout(
        &mut self,
        node: NodeId,
    ) -> Result<LayoutFound, Internal> {
        let mut previous: Option<Vec<crate::semantics::analysis::query::Query>> = None;
        loop {
            let mut need = Vec::new();
            if let Some(found) = self.layout_of(node, &mut need)? {
                return Ok(found);
            }
            if need.is_empty() || previous.as_ref() == Some(&need) {
                return Err(Internal::missed("array layout"));
            }
            for query in &need {
                self.demand(*query)?;
            }
            previous = Some(need);
        }
    }

    pub(super) fn known_arms(
        &mut self,
        op: &Op,
        arms: Vec<NodeId>,
    ) -> Result<Vec<NodeId>, Internal> {
        let mut previous: Option<Vec<crate::semantics::analysis::query::Query>> = None;
        loop {
            let mut need = Vec::new();
            if let Some(chosen) = self.chosen_arm(op, &mut need)? {
                return Ok(match chosen.and_then(|arm| arms.get(arm)) {
                    Some(arm) => vec![*arm],
                    None => arms,
                });
            }
            if need.is_empty() || previous.as_ref() == Some(&need) {
                return Err(Internal::missed("chosen arm"));
            }
            for query in &need {
                self.demand(*query)?;
            }
            previous = Some(need);
        }
    }

    fn capacity_with(&mut self, root: NodeId, env: &Env) -> Result<Piecewise, Internal> {
        self.memo.capacity_rounds += 1;
        Ok(match self.extent_with(root, env)? {
            Extent::Fixed(length) => Piecewise::constant(length),
            Extent::Variable(capacity) => capacity,
        })
    }

    pub(in crate::semantics::analysis) fn is_variable(
        &mut self,
        node: NodeId,
    ) -> Result<bool, Internal> {
        let sig = self.demand_sig(node)?;
        if !sig.reactive || sig.is_error() {
            return Ok(false);
        }
        let mut env = Env::default();
        for bound in self.memo.states.values() {
            crate::semantics::counted_walk::CountedWalk::CapacitiesIsVariable.step();
            let BoundVal::Scalar(Bound::Cell(binding)) = bound else {
                continue;
            };
            let variable = self
                .memo
                .binding_layouts
                .get(binding)
                .is_some_and(|layout| layout.variable);
            if variable && !self.memo.binding_capacities.contains_key(binding) {
                env.insert(*binding, Piecewise::constant(0));
            }
        }
        Ok(matches!(self.extent_with(node, &env)?, Extent::Variable(_)))
    }

    pub(in crate::semantics::analysis) fn extent(
        &mut self,
        node: NodeId,
    ) -> Result<Option<u32>, Internal> {
        Ok(match self.extent_with(node, &Env::default())? {
            Extent::Fixed(_) => None,
            Extent::Variable(capacity) => {
                Some(u32::try_from(capacity.start().floor().clamp(0, LIMIT)).unwrap_or(u32::MAX))
            }
        })
    }

    fn extent_with(&mut self, root: NodeId, env: &Env) -> Result<Extent, Internal> {
        let stamp = (self.memo.settle_epoch, self.memo.binding_layouts.len());
        if self.memo.extent_stamp != stamp {
            self.memo.extent_cache.clear();
            self.memo.extent_stamp = stamp;
        }
        let mut done: FastMap<NodeId, Extent> = FastMap::default();
        self.extent_into(root, env, &mut done)?;
        done.remove(&root).ok_or(Internal::missed("capacity"))
    }

    fn extent_into(
        &mut self,
        root: NodeId,
        env: &Env,
        done: &mut FastMap<NodeId, Extent>,
    ) -> Result<(), Internal> {
        let mut pending: FastSet<NodeId> = FastSet::default();
        let mut dependent: FastSet<NodeId> = FastSet::default();
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            self.memo.capacity_visits += 1;
            if done.contains_key(&node) {
                continue;
            }
            if let Some((variable, capacity)) = self.memo.extent_cache.get(&node) {
                let cached = if *variable {
                    Extent::Variable(capacity.clone())
                } else {
                    Extent::Fixed(capacity.start().floor())
                };
                done.insert(node, cached);
                continue;
            }
            let found = self.known_layout(node)?;
            let layout = match found.shaped {
                Ok(Some(layout)) => layout,
                Ok(None) | Err(_) => {
                    done.insert(node, Extent::Fixed(0));
                    continue;
                }
            };
            if !layout.variable && found.certain {
                done.insert(node, Extent::Fixed(i128::from(layout.outer().unwrap_or(0))));
                continue;
            }
            let from_env = self.reads_env(node, env)?;
            let (operands, extra) = match self.extent_parts(node, env)? {
                Parts::Leaf(value) => {
                    if from_env {
                        dependent.insert(node);
                    } else {
                        self.cache_extent(node, &value);
                    }
                    done.insert(node, value);
                    continue;
                }
                Parts::Operands(operands, extra) => (operands, extra),
            };
            if !ready {
                if !pending.insert(node) {
                    return Err(Internal::missed("capacity order"));
                }
                stack.push((node, true));
                stack.extend(operands.iter().map(|operand| (*operand, false)));
                continue;
            }
            let values: Vec<Extent> = operands
                .iter()
                .map(|operand| {
                    done.get(operand)
                        .cloned()
                        .ok_or(Internal::missed("capacity"))
                })
                .collect::<Result<_, _>>()?;
            let value = self.combined_extent(node, (&operands, &values), extra, env)?;
            if from_env || operands.iter().any(|operand| dependent.contains(operand)) {
                dependent.insert(node);
            } else {
                self.cache_extent(node, &value);
            }
            done.insert(node, value);
        }
        Ok(())
    }

    fn reads_env(&self, node: NodeId, env: &Env) -> Result<bool, Internal> {
        let op = &self.node(node)?.op;
        if let Op::Slot(_) = op {
            return Ok(true);
        }
        let binding = if let Op::Read(binding) = op {
            Some(*binding)
        } else if let Op::History { cell, .. } = op {
            self.history_cell_binding(*cell)?
        } else {
            None
        };
        let Some(binding) = binding else {
            return Ok(false);
        };
        Ok(env.contains_key(&binding)
            || self.is_state(binding)? && !self.memo.binding_capacities.contains_key(&binding))
    }

    fn cache_extent(&mut self, node: NodeId, value: &Extent) {
        let entry = match value {
            Extent::Fixed(length) => (false, Piecewise::constant(*length)),
            Extent::Variable(capacity) => (true, capacity.clone()),
        };
        self.memo.extent_cache.insert(node, entry);
    }

    fn binding_extent(&self, binding: BindingId, env: &Env) -> Option<Extent> {
        if let Some(value) = env.get(&binding) {
            return Some(Extent::Variable(value.clone()));
        }
        let layout = self.memo.binding_layouts.get(&binding)?;
        if !layout.variable {
            return Some(Extent::Fixed(i128::from(layout.outer().unwrap_or(0))));
        }
        self.memo
            .binding_capacities
            .get(&binding)
            .map(|capacity| Extent::Variable(Piecewise::constant(i128::from(*capacity))))
    }

    fn extent_parts(&mut self, node: NodeId, env: &Env) -> Result<Parts, Internal> {
        let op = self.node(node)?.op.clone();
        Ok(match op {
            Op::Read(binding) => match self.binding_extent(binding, env) {
                Some(value) => Parts::Leaf(value),
                None => Parts::Operands(vec![self.binding(binding)?.init], None),
            },
            Op::Use(operand) | Op::Chosen(operand) | Op::Annotate(_, operand) => {
                Parts::Operands(vec![operand], None)
            }
            Op::If(_, then, otherwise) => {
                Parts::Operands(self.known_arms(&op, vec![then, otherwise])?, None)
            }
            Op::Match {
                ref arms,
                otherwise,
                ..
            } => {
                let mut operands: Vec<NodeId> = arms.iter().map(|(_, value)| *value).collect();
                operands.extend(otherwise);
                Parts::Operands(self.known_arms(&op, operands)?, None)
            }
            Op::Join(lhs, rhs) => Parts::Operands(vec![lhs, rhs], None),
            Op::Slice { base, lower, upper } => {
                let mut operands = vec![base];
                for (bound, _) in lower.iter().chain(upper.iter()) {
                    operands.extend(self.bound_measures(*bound)?);
                }
                Parts::Operands(operands, None)
            }
            Op::History { cell, fallback, .. } => {
                let mut operands: Vec<NodeId> = fallback.into_iter().collect();
                let mut extra = None;
                if let Some(binding) = self.history_cell_binding(cell)? {
                    match self.binding_extent(binding, env) {
                        Some(value) => extra = Some(value),
                        None => operands.push(self.binding(binding)?.init),
                    }
                }
                Parts::Operands(operands, extra)
            }
            Op::Native { native, args } => {
                let decl = self
                    .vocabulary
                    .native(native)
                    .ok_or(Internal::missed("native function"))?;
                match &decl.signature.result {
                    NativeType::Value(ty) => match ty.capacity {
                        Some(capacity) => {
                            Parts::Leaf(Extent::Variable(Piecewise::constant(i128::from(capacity))))
                        }
                        None => Parts::Leaf(Extent::Fixed(i128::from(
                            ty.lengths.last().copied().unwrap_or(0),
                        ))),
                    },
                    NativeType::CapacityOf(_, position) => {
                        match (decl.signature.params.get(*position), args.get(*position)) {
                            (Some(NativeType::Value(ty)), _) => {
                                let rows = ty.capacity.or_else(|| ty.lengths.last().copied());
                                Parts::Leaf(Extent::Variable(Piecewise::constant(i128::from(
                                    rows.unwrap_or(0),
                                ))))
                            }
                            (Some(NativeType::Open(_)), Some(arg)) => {
                                Parts::Operands(vec![*arg], None)
                            }
                            _ => Parts::Leaf(Extent::Fixed(0)),
                        }
                    }
                    NativeType::Open(_) => Parts::Leaf(Extent::Fixed(0)),
                }
            }
            Op::ToRead { ref attrs, .. } => match attrs.first() {
                Some((item, attr)) => {
                    let decl = self
                        .attr_decl(*item, *attr)?
                        .ok_or(Internal::missed("attribute"))?;
                    Parts::Leaf(match decl.ty.capacity {
                        Some(capacity) => {
                            Extent::Variable(Piecewise::constant(i128::from(capacity)))
                        }
                        None => {
                            Extent::Fixed(i128::from(decl.ty.lengths.last().copied().unwrap_or(0)))
                        }
                    })
                }
                None => Parts::Leaf(Extent::Fixed(0)),
            },
            Op::Builtin(Builtin::Bytes, args) => Parts::Operands(args, None),
            Op::Slot(slot) => Parts::Leaf(self.slot_extent(slot)?),
            Op::For(ref for_node) => {
                let mut operands = Vec::new();
                match for_node.iterable {
                    ForIterable::Values(values) => operands.push(values),
                    ForIterable::Range(low, high) => {
                        operands.extend(self.bound_measures(low)?);
                        operands.extend(self.bound_measures(high)?);
                    }
                }
                operands.push(for_node.start);
                Parts::Operands(operands, None)
            }
            Op::Array(elements) => Parts::Leaf(Extent::Fixed(
                i128::try_from(elements.len()).unwrap_or(i128::MAX),
            )),
            _ => Parts::Leaf(Extent::Fixed(0)),
        })
    }

    fn combined_extent(
        &mut self,
        node: NodeId,
        (operands, values): (&[NodeId], &[Extent]),
        extra: Option<Extent>,
        env: &Env,
    ) -> Result<Extent, Internal> {
        let op = self.node(node)?.op.clone();
        let first = values.first().cloned().unwrap_or(Extent::Fixed(0));
        Ok(match op {
            Op::Join(..) => values
                .iter()
                .skip(1)
                .fold(first.clone(), |total, value| total.join(value)),
            Op::Slice { lower, upper, .. } => {
                let measured: FastMap<NodeId, Extent> = operands
                    .iter()
                    .copied()
                    .zip(values.iter().cloned())
                    .skip(1)
                    .collect();
                let lower = self.bound_form(lower, false, &measured)?;
                let upper = self.bound_form(upper, true, &measured)?;
                if outside_i64(lower) || outside_i64(upper) {
                    return Ok(Extent::Variable(Piecewise::constant(0)));
                }
                match (&first, fixed_slice(lower, upper, &first)) {
                    (Extent::Fixed(_), Some(length)) => Extent::Fixed(length),
                    _ => Extent::Variable(Piecewise::compose(
                        &slice_transform(lower, upper),
                        &first.capacity(),
                    )),
                }
            }
            Op::If(..) | Op::Match { .. } => values
                .iter()
                .skip(1)
                .fold(first.clone(), |widest, value| widest.choice(value)),
            Op::Native { .. } => Extent::Variable(first.capacity()),
            Op::For(ref for_node) => self.for_extent(node, for_node, (operands, values), env)?,
            Op::History { .. } => {
                let reading = match extra {
                    Some(extra) => extra,
                    None => values.last().cloned().unwrap_or(Extent::Fixed(0)),
                };
                match (&reading, values.first()) {
                    (Extent::Fixed(_), _) | (_, None) => reading,
                    (Extent::Variable(_), Some(fallback)) => reading.choice(fallback),
                }
            }
            _ => first,
        })
    }

    fn bound_form(
        &mut self,
        bound: Option<(NodeId, bool)>,
        upper: bool,
        measured: &FastMap<NodeId, Extent>,
    ) -> Result<BoundForm, Internal> {
        let Some((node, from_end)) = bound else {
            return Ok(BoundForm::Absent);
        };
        let sig = self.known_sig(node)?;
        if sig.is_error() || sig.frame {
            return Ok(BoundForm::Reactive(Extreme::Unbounded));
        }
        if sig.reactive {
            let capacity = |array: NodeId| {
                measured.get(&array).and_then(|extent| match extent {
                    Extent::Fixed(length) => Some(*length),
                    Extent::Variable(capacity) => {
                        capacity.constant_value().map(|value| value.floor())
                    }
                })
            };
            let range = self.bound_range(node, &capacity)?;
            let extreme = match (upper, from_end) {
                (true, false) => range.max.map(Extreme::Start),
                (true, true) => range.min.map(Extreme::End),
                (false, false) => range.min.map(Extreme::Start),
                (false, true) => range.max.map(Extreme::End),
            };
            return Ok(BoundForm::Reactive(extreme.unwrap_or(Extreme::Unbounded)));
        }
        Ok(match self.demand_cval(node, EvalTy::Exact)? {
            CVal::Int(value) if from_end => BoundForm::End(value),
            CVal::Int(value) => BoundForm::Start(value),
            CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Str(_)
            | CVal::Node(_)
            | CVal::Array(_)
            | CVal::Error => BoundForm::Reactive(Extreme::Unbounded),
        })
    }

    fn slot_extent(&self, slot: SlotId) -> Result<Extent, Internal> {
        if let Some(capacity) = self.memo.slot_extents.get(&slot) {
            return Ok(Extent::Variable(capacity.clone()));
        }
        let record = self.slot(slot)?;
        Ok(match &record.layout {
            None => Extent::Fixed(0),
            Some(layout) if !layout.variable => {
                Extent::Fixed(i128::from(layout.outer().unwrap_or(0)))
            }
            Some(_) => {
                let capacity = match (record.capacity, self.memo.loop_capacities.get(&slot)) {
                    (Some(annotated), _) => i128::from(annotated),
                    (None, Some(found)) => *found,
                    (None, None) => 0,
                };
                Extent::Variable(Piecewise::constant(capacity))
            }
        })
    }

    fn for_extent(
        &mut self,
        node: NodeId,
        for_node: &ForNode,
        (operands, values): (&[NodeId], &[Extent]),
        env: &Env,
    ) -> Result<Extent, Internal> {
        let accumulator = self.slot(for_node.accumulator)?.clone();
        let Some(layout) = &accumulator.layout else {
            return Ok(Extent::Fixed(0));
        };
        if !layout.variable {
            return Ok(Extent::Fixed(i128::from(layout.outer().unwrap_or(0))));
        }
        let measured: FastMap<NodeId, Extent> = operands
            .iter()
            .copied()
            .zip(values.iter().cloned())
            .collect();
        let held = |extent: Option<&Extent>| match extent {
            Some(Extent::Fixed(length)) => *length,
            Some(Extent::Variable(capacity)) => capacity.start().floor(),
            None => 0,
        };
        let count = match for_node.iterable {
            ForIterable::Values(values) => held(measured.get(&values)),
            ForIterable::Range(low, high) => {
                let capacity = |array: NodeId| {
                    measured.get(&array).and_then(|extent| match extent {
                        Extent::Fixed(length) => Some(*length),
                        Extent::Variable(capacity) => {
                            capacity.constant_value().map(|value| value.floor())
                        }
                    })
                };
                let low = self.bound_range(low, &capacity)?;
                let high = self.bound_range(high, &capacity)?;
                match (low.min, high.max) {
                    (Some(low), Some(high)) => high.saturating_sub(low).max(0),
                    (None, _) | (_, None) => 0,
                }
            }
        };
        let start = held(measured.get(&for_node.start));
        self.memo
            .slot_extents
            .insert(for_node.accumulator, Piecewise::affine(0, 1));
        let function = self.capacity_with(for_node.body, env);
        self.memo.slot_extents.remove(&for_node.accumulator);
        let function = function?;
        let mut current = start;
        let mut most = start;
        let mut left = count;
        while left > 0 && current < LIMIT {
            let at = Ratio::int(current);
            let next = function.at(at).floor();
            if next <= current {
                break;
            }
            let growth = next - current;
            let steps = match function.segment_at(at) {
                (slope, Some(end)) if slope == Ratio::int(1) => (end - at)
                    .div(Ratio::int(growth))
                    .map_or(1, |steps| steps.floor().max(1)),
                (slope, None) if slope == Ratio::int(1) => left,
                (_, _) => 1,
            }
            .min(left);
            current = current.saturating_add(growth.saturating_mul(steps));
            most = most.max(current);
            left -= steps;
        }
        self.memo.loop_capacities.insert(for_node.accumulator, most);
        if most >= LIMIT && self.memo.loop_reports.insert(node) {
            self.error_at_node(
                Code::LengthLimit,
                node,
                format!(
                    "this `for` expression holds up to {most} elements, and an array's length and capacity are below 2³¹; a slice in its body, such as `[^8..]`, keeps the newest eight"
                ),
            )?;
        }
        let Some(annotated) = accumulator.capacity else {
            return Ok(Extent::Variable(Piecewise::constant(most.min(LIMIT))));
        };
        let annotated = i128::from(annotated);
        if most > annotated && self.memo.loop_reports.insert(node) {
            self.error_at_node(
                Code::CapacityExceeded,
                node,
                format!(
                    "this `for` expression holds up to {most} elements, more than the annotated capacity {annotated} of its accumulator"
                ),
            )?;
        }
        Ok(Extent::Variable(Piecewise::constant(annotated)))
    }

    fn range_bounds(
        &mut self,
        node: NodeId,
        (low, high): (NodeId, NodeId),
    ) -> Result<(), Internal> {
        let scope = self.node(node)?.scope;
        if self.in_probe(scope)? || !self.known_sig(node)?.reactive {
            return Ok(());
        }
        let mut measured: FastMap<NodeId, i128> = FastMap::default();
        for bound in [low, high] {
            for array in self.bound_measures(bound)? {
                let capacity = match self.extent_with(array, &Env::default())? {
                    Extent::Fixed(length) => length,
                    Extent::Variable(capacity) => capacity.start().floor(),
                };
                measured.insert(array, capacity);
            }
        }
        let capacity = |array: NodeId| measured.get(&array).copied();
        for (bound, upper) in [(low, false), (high, true)] {
            let sig = self.known_sig(bound)?;
            if !sig.reactive || sig.is_error() {
                continue;
            }
            let range = self.bound_range(bound, &capacity)?;
            let (side, example) = if upper {
                (range.max, "0..min(n, 16)")
            } else {
                (range.min, "max(n, 0)..16")
            };
            if side.is_some() {
                continue;
            }
            let message = if upper {
                "this upper bound of the range reads a cell, and its form gives it no maximum"
            } else {
                "this lower bound of the range reads a cell, and its form gives it no minimum"
            };
            let (loc, scope) = {
                let record = self.node(bound)?;
                (record.loc, record.scope)
            };
            self.error_helped(
                Code::RangeBound,
                loc,
                scope,
                message.to_string(),
                format!("bound it by its form, as in `{example}`"),
            )?;
        }
        Ok(())
    }
}
