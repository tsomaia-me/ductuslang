use std::collections::{HashMap, HashSet, VecDeque};

use super::bound_form::BoundForm;
use super::core::Core;
use super::core_value::CoreValue;
use super::cycles::strongly_connected;
use super::decl_ref::DeclRef;
use super::extent::{Extent, LIMIT, fixed_slice, outside_i64};
use super::extreme::Extreme;
use super::folds::ANY_LENGTH;
use super::leaf::Leaf;
use super::piecewise::Piecewise;
use super::ratio::Ratio;
use super::slice_transform::slice_transform;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::{EvalTy, Value};

const CEILING: i128 = 1 << 120;

const STEPS: u64 = 1 << 20;

const FOLD_HELP: &str = "slice the accumulator in the body to bound it, as in `(all + [x])[^8..]`";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Checking {
    Nothing,
    Joins,
    Everything,
}

struct Reach {
    node: u32,
    inst: u32,
    chain: Vec<u32>,
    total: i128,
    fold: bool,
}

struct Walk {
    cells: HashMap<u32, Piecewise>,
    frames: Vec<(u32, Vec<Option<Extent>>)>,
    checking: Checking,
    reached: Vec<Reach>,
}

impl Walk {
    fn new(checking: Checking) -> Walk {
        Walk {
            cells: HashMap::new(),
            frames: Vec::new(),
            checking,
            reached: Vec::new(),
        }
    }

    fn holding(cells: &[u32], values: impl Iterator<Item = Piecewise>) -> Walk {
        let mut walk = Walk::new(Checking::Nothing);
        walk.cells = cells.iter().copied().zip(values).collect();
        walk
    }
}

enum Parts {
    Leaf(Extent),
    Operands(Vec<u32>),
}

enum Solved {
    Capacities(Vec<i128>),
    Limit(Vec<i128>),
    Unbounded(Vec<bool>),
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

fn grown(function: &Piecewise, start: i128, count: i128) -> i128 {
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
    most
}

fn joined(total: i128) -> String {
    format!(
        "this join holds up to {total} elements, and an array's length and capacity are below 2³¹"
    )
}

impl Core<'_> {
    pub fn settle_capacities(&mut self) -> Result<(), Internal> {
        let mark = self.diagnostics.len();
        let outer = std::mem::replace(&mut self.inst, NONE);
        let settled = self.solve_cells();
        self.inst = outer;
        let covered = settled?;
        let outer = std::mem::replace(&mut self.inst, NONE);
        let checked = self.length_limits(&covered);
        self.inst = outer;
        checked?;
        let outer = std::mem::replace(&mut self.inst, NONE);
        let annotated = self.annotated_capacities();
        self.inst = outer;
        annotated?;
        if let Some((at, _)) = self.stop.as_mut()
            && *at < mark
        {
            let drawn: Vec<_> = self.diagnostics.drain(mark..).collect();
            let count = drawn.len();
            let at_now = *at;
            self.diagnostics.splice(at_now..at_now, drawn);
            *at += count;
        }
        Ok(())
    }

    fn variable_cell(&self, at: u32) -> Result<bool, Internal> {
        if !self.derived(at) || self.is_cyclic_cell(at) || self.cell_annotation_ty(at)?.is_some() {
            return Ok(false);
        }
        if self.group_variable.contains(&at) {
            return Ok(true);
        }
        let root = self.cell_root(at)?;
        if !self.has_fact(root)? {
            return Ok(false);
        }
        let fact = self.fact(root)?;
        if fact.dims.is_empty() || matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(false);
        }
        Ok(fact.dims.first() == Some(&ANY_LENGTH) || self.variable_length(root)?)
    }

    fn solve_cells(&mut self) -> Result<HashSet<u32>, Internal> {
        let count = u32::try_from(self.ir.cells.len()).unwrap_or(0);
        let mut members = Vec::new();
        for at in 0..count {
            CountedWalk::CoreCapacity.step();
            if self.variable_cell(at)? {
                members.push(at);
            }
        }
        let positions: HashMap<u32, u32> = members
            .iter()
            .enumerate()
            .map(|(position, at)| (*at, u32::try_from(position).unwrap_or(NONE)))
            .collect();
        let edges: Vec<Vec<u32>> = members
            .iter()
            .map(|at| {
                let mut targets: Vec<u32> = self
                    .cell_reads
                    .get(*at as usize)
                    .map(|reads| {
                        reads
                            .present
                            .iter()
                            .copied()
                            .chain(reads.history.iter().map(|(target, _)| *target))
                            .filter_map(|target| positions.get(&target).copied())
                            .collect()
                    })
                    .unwrap_or_default();
                targets.sort_unstable();
                targets.dedup();
                targets
            })
            .collect();
        let mut covered = HashSet::new();
        for component in strongly_connected(&edges) {
            let group: Vec<u32> = component
                .iter()
                .filter_map(|position| members.get(*position as usize).copied())
                .collect();
            match self.solve(&group)? {
                Solved::Capacities(capacities) => {
                    for (at, capacity) in group.iter().zip(capacities) {
                        self.cell_capacities.insert(*at, capacity);
                    }
                    continue;
                }
                Solved::Limit(reached) => {
                    self.limited(&group, &reached)?;
                    for at in &group {
                        self.cell_capacities.insert(*at, 0);
                    }
                }
                Solved::Unbounded(growing) => {
                    let first = group
                        .iter()
                        .zip(&growing)
                        .filter(|(_, grows)| **grows)
                        .map(|(at, _)| *at)
                        .min();
                    if let Some(at) = first {
                        self.unbounded(at)?;
                    }
                    for at in &group {
                        self.cell_capacities.insert(*at, 0);
                    }
                }
            }
            covered.extend(group);
        }
        Ok(covered)
    }

    fn solve(&mut self, members: &[u32]) -> Result<Solved, Internal> {
        let roots: Vec<u32> = members
            .iter()
            .map(|at| self.cell_root(*at))
            .collect::<Result<_, _>>()?;
        let longest = members.len() + 1;
        let mut history: VecDeque<Vec<i128>> = VecDeque::from([vec![0i128; members.len()]]);
        let mut streak: Option<Streak> = None;
        for _ in 0..STEPS {
            let current = history
                .back()
                .cloned()
                .ok_or(Internal::missed("capacity"))?;
            let next = self.capacity_step(members, &roots, &current)?;
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
            let run = self.phase_run(members, &roots, (&current, &next), &direction)?;
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
        members: &[u32],
        roots: &[u32],
        current: &[i128],
    ) -> Result<Vec<i128>, Internal> {
        let mut walk = Walk::holding(
            members,
            current.iter().map(|value| Piecewise::constant(*value)),
        );
        roots
            .iter()
            .map(|root| {
                Ok(self
                    .extent_in(*root, &mut walk, &mut HashMap::new())?
                    .capacity()
                    .start()
                    .floor())
            })
            .collect()
    }

    fn phase_run(
        &mut self,
        members: &[u32],
        roots: &[u32],
        (current, next): (&[i128], &[i128]),
        direction: &[i128],
    ) -> Result<Option<Ratio>, Internal> {
        let mut walk = Walk::holding(
            members,
            current
                .iter()
                .zip(direction)
                .map(|(start, slope)| Piecewise::affine(*start, *slope)),
        );
        let mut run: Option<Ratio> = None;
        for ((root, after), slope) in roots.iter().zip(next).zip(direction) {
            let function = self
                .extent_in(*root, &mut walk, &mut HashMap::new())?
                .capacity();
            let shifted = function.add(&Piecewise::affine(*after, *slope).neg());
            if let Some(length) = shifted.zero_run() {
                run = Some(run.map_or(length, |shortest| shortest.min(length)));
            }
        }
        Ok(run)
    }

    fn capacity_name(&self, at: u32) -> Result<(String, u32), Internal> {
        let rec = self
            .ir
            .cells
            .get(at as usize)
            .ok_or(Internal::missed("IR cell"))?;
        let name = self.ir.binder(rec.binder)?.name;
        let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
        Ok((self.ir.symbols.text(entry.sym).to_string(), name))
    }

    fn limited(&mut self, group: &[u32], reached: &[i128]) -> Result<(), Internal> {
        let mut walk = Walk::holding(
            group,
            reached.iter().map(|value| Piecewise::constant(*value)),
        );
        walk.checking = Checking::Joins;
        for at in group {
            let root = self.cell_root(*at)?;
            self.extent_in(root, &mut walk, &mut HashMap::new())?;
        }
        if !walk.reached.is_empty() {
            return self.draw_reaches(std::mem::take(&mut walk.reached));
        }
        let first = group
            .iter()
            .zip(reached)
            .filter(|(_, value)| **value >= LIMIT)
            .min_by_key(|(at, _)| **at);
        if let Some((at, value)) = first {
            let (name, _) = self.capacity_name(*at)?;
            let root = self.cell_root(*at)?;
            self.draw_at(
                Code::LengthLimit,
                root,
                format!(
                    "`{name}` keeps up to {value} elements, and an array's length and capacity are below 2³¹"
                ),
            )?;
        }
        Ok(())
    }

    fn unbounded(&mut self, at: u32) -> Result<(), Internal> {
        let (text, name) = self.capacity_name(at)?;
        let file = self.file_of(self.cell_root(at)?);
        let span = self.name_span(file, name)?;
        self.draw(
            Code::UnboundedState,
            span,
            format!(
                "`{text}` grows at every step, so no capacity holds it; a slice such as `[^8..]` around its rule keeps the newest eight elements"
            ),
        );
        Ok(())
    }

    fn draw_reaches(&mut self, mut reaches: Vec<Reach>) -> Result<(), Internal> {
        let mut keyed = Vec::with_capacity(reaches.len());
        for reach in reaches.drain(..) {
            let key = (self.file_of(reach.node), self.ir.span(reach.node)?.0);
            keyed.push((key, reach));
        }
        keyed.sort_by_key(|(key, _)| *key);
        for (_, reach) in keyed {
            let outer = std::mem::replace(&mut self.inst, reach.inst);
            let chain = std::mem::replace(&mut self.eval_chain, reach.chain);
            let drawn = if reach.fold {
                self.draw_helped_at(
                    Code::LengthLimit,
                    reach.node,
                    format!(
                        "this `for` expression holds up to {} elements, and an array's length and capacity are below 2³¹",
                        reach.total
                    ),
                    FOLD_HELP,
                )
            } else {
                self.draw_at(Code::LengthLimit, reach.node, joined(reach.total))
            };
            self.eval_chain = chain;
            self.inst = outer;
            drawn?;
        }
        Ok(())
    }

    fn checked_root(&self, node: u32) -> Result<bool, Internal> {
        let (op, _, _, _) = self.ir.node(node)?;
        let array = match op {
            Op::Add | Op::For => true,
            Op::Call | Op::NamedCall => false,
            Op::Name
            | Op::Slice
            | Op::If
            | Op::Past
            | Op::Array
            | Op::Int
            | Op::Float
            | Op::Bool
            | Op::Str
            | Op::Variant
            | Op::To
            | Op::Field
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Stepped
            | Op::Match
            | Op::Index
            | Op::IndexFromEnd
            | Op::Neg
            | Op::Not
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IntDiv
            | Op::Rem
            | Op::Pow
            | Op::And
            | Op::Or
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::Is
            | Op::IsNot
            | Op::Convert
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => return Ok(false),
        };
        if !self.has_fact(node)? || !self.is_reactive(node)? {
            return Ok(false);
        }
        Ok(!array || !self.fact(node)?.dims.is_empty())
    }

    fn length_limits(&mut self, covered: &HashSet<u32>) -> Result<(), Internal> {
        let mut skipped = HashSet::new();
        for at in covered {
            skipped.extend(self.postorder(self.cell_root(*at)?)?);
        }
        let mut walk = Walk::new(Checking::Everything);
        let mut done = HashMap::new();
        let first_body_owner = self.functions() + u32::try_from(self.ir.fors.len()).unwrap_or(0);
        let mut owned: HashMap<u32, Vec<u32>> = HashMap::new();
        for local in 0..self.ir.op.len() {
            CountedWalk::CoreCapacity.step();
            let node = self.ir.base + u32::try_from(local).unwrap_or(0);
            let owner = self.body_of.get(local).copied().unwrap_or(NONE);
            if owner != NONE {
                if owner >= first_body_owner {
                    owned.entry(owner).or_default().push(node);
                }
                continue;
            }
            if skipped.contains(&node) || !self.checked_root(node)? {
                continue;
            }
            self.extent_in(node, &mut walk, &mut done)?;
        }
        for inst in 0..u32::try_from(self.bodies.len()).unwrap_or(0) {
            let owner = self
                .bodies
                .get(inst as usize)
                .map_or(NONE, |body| body.function);
            let Some(nodes) = owned.get(&owner).cloned() else {
                continue;
            };
            self.inst = inst;
            let mut done = HashMap::new();
            for node in nodes {
                if self.checked_root(node)? {
                    self.extent_in(node, &mut walk, &mut done)?;
                }
            }
            self.inst = NONE;
        }
        self.draw_reaches(std::mem::take(&mut walk.reached))
    }

    fn extent_in(
        &mut self,
        root: u32,
        walk: &mut Walk,
        done: &mut HashMap<u32, Extent>,
    ) -> Result<Extent, Internal> {
        let mut parts: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut dependent: HashSet<u32> = HashSet::new();
        let caching = walk.checking == Checking::Nothing && walk.frames.is_empty();
        let mut stack = vec![(root, false)];
        while let Some((node, ready)) = stack.pop() {
            CountedWalk::CoreCapacity.step();
            if done.contains_key(&node) {
                continue;
            }
            if ready {
                let operands = parts.remove(&node).unwrap_or_default();
                let values: Vec<Extent> = operands
                    .iter()
                    .map(|operand| done.get(operand).cloned().unwrap_or(Extent::Fixed(0)))
                    .collect();
                let extent = self.combined_extent(node, &values, walk)?;
                if operands.iter().any(|operand| dependent.contains(operand))
                    || self.env_read(node, walk)?
                {
                    dependent.insert(node);
                } else if caching && self.top_level(node)? {
                    self.extent_cache.insert(node, extent.clone());
                }
                done.insert(node, extent);
                continue;
            }
            if parts.contains_key(&node) {
                done.insert(node, Extent::Fixed(0));
                continue;
            }
            if caching && let Some(cached) = self.extent_cache.get(&node) {
                done.insert(node, cached.clone());
                continue;
            }
            match self.extent_parts(node, walk)? {
                Parts::Leaf(extent) => {
                    if self.env_read(node, walk)? {
                        dependent.insert(node);
                    }
                    done.insert(node, extent);
                }
                Parts::Operands(operands) => {
                    stack.push((node, true));
                    stack.extend(operands.iter().rev().map(|operand| (*operand, false)));
                    parts.insert(node, operands);
                }
            }
        }
        Ok(done.get(&root).cloned().unwrap_or(Extent::Fixed(0)))
    }

    fn extent_parts(&mut self, node: u32, walk: &Walk) -> Result<Parts, Internal> {
        if !self.has_fact(node)? {
            return Ok(Parts::Leaf(Extent::Fixed(0)));
        }
        let fact = self.fact(node)?;
        let Some(length) = fact.dims.first().copied() else {
            return Ok(Parts::Leaf(Extent::Fixed(0)));
        };
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Parts::Leaf(Extent::Fixed(0)));
        }
        if !self.is_reactive(node)? {
            return Ok(Parts::Leaf(Extent::open(length)));
        }
        let (op, a, b, c) = self.ir.node(node)?;
        Ok(match op {
            Op::Name => self.name_parts(node, length, walk)?,
            Op::Add => Parts::Operands(vec![a, b]),
            Op::Slice => Parts::Operands(vec![a]),
            Op::If => Parts::Operands(match self.chosen(a)? {
                Some(true) => vec![b],
                Some(false) => vec![c],
                None => vec![b, c],
            }),
            Op::Past => {
                let DeclRef::Cell(at) = self.bound(a)? else {
                    return Ok(Parts::Leaf(Extent::open(length)));
                };
                let mut operands: Vec<u32> = [c].into_iter().filter(|at| *at != NONE).collect();
                if let Parts::Operands(rule) = self.cell_parts(at, walk)? {
                    operands.extend(rule);
                }
                Parts::Operands(operands)
            }
            Op::Call | Op::NamedCall => Parts::Operands(self.arguments(node)?),
            Op::For => Parts::Operands(self.fold_operands(node)?),
            Op::Array => Parts::Leaf(Extent::Fixed(i128::from(b))),
            Op::Int
            | Op::Float
            | Op::Bool
            | Op::Str
            | Op::Variant
            | Op::To
            | Op::Field
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Stepped
            | Op::Match
            | Op::Index
            | Op::IndexFromEnd
            | Op::Neg
            | Op::Not
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IntDiv
            | Op::Rem
            | Op::Pow
            | Op::And
            | Op::Or
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::Is
            | Op::IsNot
            | Op::Convert
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => Parts::Leaf(Extent::open(length)),
        })
    }

    fn chosen(&mut self, condition: u32) -> Result<Option<bool>, Internal> {
        if self.is_reactive(condition)? {
            return Ok(None);
        }
        Ok(match self.quiet_value(condition, EvalTy::Bool)? {
            Some(Value::Bool(taken)) => Some(taken),
            Some(_) | None => None,
        })
    }

    fn quiet_value(&mut self, node: u32, ty: EvalTy) -> Result<Option<CoreValue>, Internal> {
        let chain = std::mem::take(&mut self.eval_chain);
        let (drawn, stop) = (self.diagnostics.len(), self.stop.clone());
        let value = self.evaluate(node, ty);
        self.diagnostics.truncate(drawn);
        self.stop = stop;
        self.eval_chain = chain;
        Ok(value?.ok())
    }

    fn name_parts(&self, node: u32, length: u32, walk: &Walk) -> Result<Parts, Internal> {
        Ok(match self.bound(node)? {
            DeclRef::Cell(at) => self.cell_parts(at, walk)?,
            DeclRef::Let(at) => Parts::Operands(vec![self.let_root(at)?]),
            DeclRef::Param(position) => {
                let owner = self.binding_owner(node)?;
                let found = walk
                    .frames
                    .iter()
                    .rev()
                    .find(|(frame, _)| *frame == owner)
                    .and_then(|(_, params)| params.get(position as usize).cloned().flatten());
                Parts::Leaf(found.unwrap_or(Extent::open(length)))
            }
            DeclRef::None
            | DeclRef::Instance(_)
            | DeclRef::Duplicate
            | DeclRef::Fn(_)
            | DeclRef::Pattern(_) => Parts::Leaf(Extent::open(length)),
        })
    }

    fn cell_parts(&self, at: u32, walk: &Walk) -> Result<Parts, Internal> {
        if let Some(capacity) = walk.cells.get(&at) {
            return Ok(Parts::Leaf(Extent::Variable(capacity.clone())));
        }
        if let Some(capacity) = self.cell_capacity_annotation(at)? {
            return Ok(Parts::Leaf(Extent::Variable(Piecewise::constant(
                i128::from(capacity),
            ))));
        }
        if !self.derived(at) {
            return Ok(Parts::Leaf(self.fixed_cell(at)?));
        }
        if let Some(capacity) = self.cell_capacities.get(&at) {
            return Ok(Parts::Leaf(Extent::Variable(Piecewise::constant(
                *capacity,
            ))));
        }
        if !self.variable_cell(at)? {
            return Ok(Parts::Leaf(self.fixed_cell(at)?));
        }
        Ok(Parts::Operands(vec![self.cell_root(at)?]))
    }

    fn fixed_cell(&self, at: u32) -> Result<Extent, Internal> {
        let root = self.cell_root(at)?;
        if !self.has_fact(root)? {
            return Ok(Extent::Fixed(0));
        }
        let fact = self.cell_read_fact(at)?;
        Ok(match fact.dims.first() {
            Some(length) if *length != ANY_LENGTH => Extent::Fixed(i128::from(*length)),
            Some(_) => Extent::Variable(Piecewise::constant(0)),
            None => Extent::Fixed(0),
        })
    }

    fn combined_extent(
        &mut self,
        node: u32,
        values: &[Extent],
        walk: &mut Walk,
    ) -> Result<Extent, Internal> {
        let (op, a, _, c) = self.ir.node(node)?;
        let first = values.first().cloned().unwrap_or(Extent::Fixed(0));
        Ok(match op {
            Op::Add => {
                let total = values
                    .iter()
                    .skip(1)
                    .fold(first, |total, value| total.join(value));
                if walk.checking != Checking::Nothing
                    && total.reaches_limit()
                    && !values.iter().any(Extent::reaches_limit)
                {
                    walk.reached.push(Reach {
                        node,
                        inst: self.inst,
                        chain: self.eval_chain.clone(),
                        total: total.held(),
                        fold: false,
                    });
                }
                total
            }
            Op::Slice => self.slice_extent(node, &first)?,
            Op::If => values
                .iter()
                .skip(1)
                .fold(first, |widest, value| widest.choice(value)),
            Op::Past => {
                let DeclRef::Cell(at) = self.bound(a)? else {
                    return Ok(first);
                };
                let reading = match self.cell_parts(at, walk)? {
                    Parts::Leaf(extent) => extent,
                    Parts::Operands(_) => values.last().cloned().unwrap_or(Extent::Fixed(0)),
                };
                match (&reading, c != NONE) {
                    (Extent::Variable(_), true) => reading.choice(&first),
                    (Extent::Fixed(_) | Extent::Variable(_), _) => reading,
                }
            }
            Op::Call | Op::NamedCall => self.call_extent(node, values, walk)?,
            Op::For => self.fold_extent(node, values, walk)?,
            Op::Name
            | Op::Array
            | Op::Int
            | Op::Float
            | Op::Bool
            | Op::Str
            | Op::Variant
            | Op::To
            | Op::Field
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Stepped
            | Op::Match
            | Op::Index
            | Op::IndexFromEnd
            | Op::Neg
            | Op::Not
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IntDiv
            | Op::Rem
            | Op::Pow
            | Op::And
            | Op::Or
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::Is
            | Op::IsNot
            | Op::Convert
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => first,
        })
    }

    fn slice_extent(&mut self, node: u32, base: &Extent) -> Result<Extent, Internal> {
        let (_, _, _, flags) = self.ir.node(node)?;
        let [lower, upper] = self.bounds(node)?;
        let lower = self.bound_form(lower, false, flags & 1 != 0)?;
        let upper = self.bound_form(upper, true, flags & 2 != 0)?;
        if outside_i64(lower) || outside_i64(upper) {
            return Ok(Extent::Variable(Piecewise::constant(0)));
        }
        Ok(match fixed_slice(lower, upper, base) {
            Some(length) => Extent::Fixed(length),
            None => Extent::Variable(Piecewise::compose(
                &slice_transform(lower, upper),
                &base.capacity(),
            )),
        })
    }

    fn bound_form(
        &mut self,
        bound: u32,
        upper: bool,
        from_end: bool,
    ) -> Result<BoundForm, Internal> {
        if bound == NONE {
            return Ok(BoundForm::Absent);
        }
        if !self.is_reactive(bound)? {
            return Ok(match self.quiet_value(bound, EvalTy::Exact)? {
                Some(Value::Int(value)) if from_end => BoundForm::End(value),
                Some(Value::Int(value)) => BoundForm::Start(value),
                Some(_) | None => BoundForm::Reactive(Extreme::Unbounded),
            });
        }
        let range = self.bound_extremes(bound)?;
        let extreme = match (upper, from_end) {
            (true, false) => range.max.map(Extreme::Start),
            (true, true) => range.min.map(Extreme::End),
            (false, false) => range.min.map(Extreme::Start),
            (false, true) => range.max.map(Extreme::End),
        };
        Ok(BoundForm::Reactive(extreme.unwrap_or(Extreme::Unbounded)))
    }

    fn call_extent(
        &mut self,
        node: u32,
        values: &[Extent],
        walk: &mut Walk,
    ) -> Result<Extent, Internal> {
        let length = self.fact(node)?.dims.first().copied().unwrap_or(0);
        let Some(body) = self.body_of_call(node) else {
            return Ok(Extent::open(length));
        };
        let function = self
            .bodies
            .get(body as usize)
            .map_or(NONE, |record| record.function);
        let value = self.body_value(body)?;
        let outer = std::mem::replace(&mut self.inst, body);
        walk.frames
            .push((function, values.iter().cloned().map(Some).collect()));
        self.eval_chain.push(node);
        let found = self.extent_in(value, walk, &mut HashMap::new());
        self.eval_chain.pop();
        walk.frames.pop();
        self.inst = outer;
        found
    }

    fn fold_extent(
        &mut self,
        node: u32,
        values: &[Extent],
        walk: &mut Walk,
    ) -> Result<Extent, Internal> {
        let length = self.fact(node)?.dims.first().copied().unwrap_or(0);
        if length != ANY_LENGTH {
            return Ok(Extent::Fixed(i128::from(length)));
        }
        let (_, record, _, _) = self.ir.node(node)?;
        let rec = self.fold_rec(node)?;
        let start = values.first().map_or(0, Extent::held);
        let count = if rec.range {
            let low = self.bound_extremes(rec.iterable.0)?;
            let high = self.bound_extremes(rec.iterable.1)?;
            match (low.min, high.max) {
                (Some(low), Some(high)) => high.saturating_sub(low).max(0),
                (None, _) | (_, None) => 0,
            }
        } else {
            values.get(1).map_or(0, Extent::held)
        };
        let Some(body) = self.body_of_call(node) else {
            return Ok(Extent::Variable(Piecewise::constant(start)));
        };
        let owner = self.functions() + record;
        let position = (rec.names.1 - rec.names.0) as usize;
        let at = (body, owner, position, rec.body);
        let function = self
            .fold_body_extent(
                at,
                Extent::Variable(Piecewise::affine(0, 1)),
                walk,
                Checking::Nothing,
            )?
            .capacity();
        let most = grown(&function, start, count);
        if let Some(annotated) = self.binder_capacity(rec.accumulator)? {
            let annotated = i128::from(annotated);
            if walk.checking == Checking::Everything && most > annotated {
                let span = self.node_span(node)?;
                let seen = self.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == Code::CapacityExceeded && diagnostic.span == span
                });
                if !seen {
                    self.draw_helped_at(
                        Code::CapacityExceeded,
                        node,
                        format!(
                            "this `for` expression holds up to {most} elements, more than the annotated capacity {annotated} of its accumulator"
                        ),
                        crate::diagnostics::accumulator_capacity_help(annotated, most),
                    )?;
                }
            }
            return Ok(Extent::Variable(Piecewise::constant(annotated)));
        }
        if walk.checking == Checking::Everything {
            let entered = grown(&function, start, count.saturating_sub(1).max(0));
            self.fold_body_extent(
                at,
                Extent::Variable(Piecewise::constant(entered)),
                walk,
                Checking::Everything,
            )?;
            if most >= LIMIT {
                walk.reached.push(Reach {
                    node,
                    inst: self.inst,
                    chain: self.eval_chain.clone(),
                    total: most,
                    fold: true,
                });
            }
        }
        Ok(Extent::Variable(Piecewise::constant(most)))
    }

    fn fold_body_extent(
        &mut self,
        (body, owner, position, root): (u32, u32, usize, u32),
        accumulator: Extent,
        walk: &mut Walk,
        checking: Checking,
    ) -> Result<Extent, Internal> {
        let mut params = vec![None; position + 1];
        if let Some(slot) = params.get_mut(position) {
            *slot = Some(accumulator);
        }
        let outer = std::mem::replace(&mut self.inst, body);
        let saved = std::mem::replace(&mut walk.checking, checking);
        walk.frames.push((owner, params));
        let found = self.extent_in(root, walk, &mut HashMap::new());
        walk.frames.pop();
        walk.checking = saved;
        self.inst = outer;
        found
    }
}

impl Core<'_> {
    fn annotated_capacities(&mut self) -> Result<(), Internal> {
        let count = u32::try_from(self.ir.cells.len()).unwrap_or(0);
        for at in 0..count {
            CountedWalk::CoreCapacity.step();
            if !self.derived(at) || self.is_cyclic_cell(at) {
                continue;
            }
            let Some(capacity) = self.cell_capacity_annotation(at)? else {
                continue;
            };
            let root = self.cell_root(at)?;
            if !self.has_fact(root)? {
                continue;
            }
            let fact = self.fact(root)?;
            if fact.dims.len() != 1 || matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
                continue;
            }
            let mut walk = Walk::new(Checking::Nothing);
            let holds = self.extent_in(root, &mut walk, &mut HashMap::new())?.held();
            let capacity = i128::from(capacity);
            if holds <= capacity || holds >= LIMIT {
                continue;
            }
            let (name, _) = self.capacity_name(at)?;
            let span = self.cell_declaration_span(at)?;
            let help = if capacity == 1 {
                String::from("slice the value to its newest element with `[^1..]`")
            } else {
                format!("slice the value to its newest {capacity} elements with `[^{capacity}..]`")
            };
            let diagnostic = crate::diagnostics::Diagnostic::new(
                Code::CapacityExceeded,
                span,
                format!(
                    "`{name}` holds up to {holds} elements, more than its annotated capacity {capacity}"
                ),
            )
            .with_help(help);
            let diagnostic = self.chained(diagnostic);
            self.push_drawn(diagnostic);
        }
        Ok(())
    }
}

impl Core<'_> {
    fn top_level(&self, node: u32) -> Result<bool, Internal> {
        Ok(self.body_of.get(self.local(node)?).copied().unwrap_or(NONE) == NONE)
    }

    fn env_read(&self, node: u32, walk: &Walk) -> Result<bool, Internal> {
        let (op, a, _, _) = self.ir.node(node)?;
        let target = match op {
            Op::Name => node,
            Op::Past => a,
            Op::Call | Op::NamedCall | Op::For => return Ok(!walk.cells.is_empty()),
            Op::Int
            | Op::Float
            | Op::Bool
            | Op::Str
            | Op::Variant
            | Op::To
            | Op::Field
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Stepped
            | Op::If
            | Op::Match
            | Op::Array
            | Op::Index
            | Op::IndexFromEnd
            | Op::Slice
            | Op::Neg
            | Op::Not
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IntDiv
            | Op::Rem
            | Op::Pow
            | Op::And
            | Op::Or
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::Is
            | Op::IsNot
            | Op::Convert
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => return Ok(false),
        };
        Ok(match self.bound(target)? {
            DeclRef::Cell(at) => walk.cells.contains_key(&at),
            DeclRef::Param(_) => true,
            DeclRef::None
            | DeclRef::Let(_)
            | DeclRef::Instance(_)
            | DeclRef::Duplicate
            | DeclRef::Fn(_)
            | DeclRef::Pattern(_) => false,
        })
    }
}
