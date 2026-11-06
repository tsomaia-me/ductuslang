use symphonyscript_core::Span;

use super::annotation::{Refusal, annotated_fact, refusal};
use super::body_instance::BodyInstance;
use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::env::Env;
use super::fact::Fact;
use super::folds::{ANY_LENGTH, ELEMENT_SHAPE, ELEMENT_TYPE, RANGE_TYPE};
use super::leaf::Leaf;
use super::loop_run::LoopRun;
use super::reason::Reason;
use super::width::is_fixed_float;
use crate::diagnostics::{Code, Diagnostic};
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{
    AttrForm, FlatIr, ForBlockRec, ItemRec, Iteration, NONE, Op, PlacementRec, WireRec,
};
use crate::semantics::operators::{EvalTy, Value, convert_to};
use crate::semantics::reporting::grouped;

const NOT_A_CELL: u8 = 0;

const THROUGH_A_LET: u8 = 1;

const DIRECTLY: u8 = 2;

const THROUGH_A_PATH: u8 = 3;

const ITERABLE: &str = "a `for` iterates over a range or an array";

const REACTIVE: &str =
    "a placement `for` iterates over a compile-time value, and this one reads a cell";

const REACTIVE_BOUND: &str =
    "a placement `for` iterates over a compile-time value, and this bound reads a cell";

const RANGE_NAMES: &str =
    "a range binds one name; two names iterate an array, its positions and its elements";

pub(crate) fn loop_items(ir: &FlatIr) -> Vec<u32> {
    ir.items
        .iter()
        .enumerate()
        .filter(|(_, item)| matches!(item, ItemRec::For(_)))
        .filter_map(|(at, _)| u32::try_from(at).ok())
        .collect()
}

pub(crate) fn iterable_roots(head: Iteration) -> Vec<u32> {
    let mut roots = vec![head.iterable.0];
    if head.range {
        roots.push(head.iterable.1);
    }
    roots.retain(|root| *root != NONE);
    roots
}

pub(crate) fn entry_roots(ir: &FlatIr, attrs: (u32, u32), roots: &mut Vec<u32>) {
    let entries = ir
        .attr_entries
        .get(attrs.0 as usize..attrs.1 as usize)
        .unwrap_or_default();
    for entry in entries {
        if let AttrForm::Value(value) = entry.form {
            roots.push(value);
        }
    }
}

pub(crate) fn loop_body(ir: &FlatIr, block: u32) -> Result<(Vec<u32>, Vec<u32>), Internal> {
    let mut roots = Vec::new();
    let mut nested = Vec::new();
    let mut blocks = vec![block];
    while let Some(block) = blocks.pop() {
        let Some(rec) = ir.blocks.get(block as usize) else {
            continue;
        };
        let mut items: Vec<u32> = ir
            .block_items
            .get(rec.items.0 as usize..rec.items.1 as usize)
            .unwrap_or_default()
            .to_vec();
        items.extend(member_items(ir, rec.members));
        while let Some(item) = items.pop() {
            match *ir.item(item)? {
                ItemRec::Placement(placement) => {
                    roots.extend([placement.default, placement.gate]);
                    entry_roots(ir, placement.attrs, &mut roots);
                    blocks.push(placement.body);
                }
                ItemRec::Wire(wire) => {
                    roots.extend([wire.default, wire.gate, wire.destination]);
                    entry_roots(ir, wire.attrs, &mut roots);
                }
                ItemRec::For(rec) => {
                    roots.extend(iterable_roots(rec.head));
                    nested.push(item);
                }
                ItemRec::Line(members) => {
                    items.extend(
                        ir.members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default(),
                    );
                }
                ItemRec::Paren(inner) if inner != NONE => items.push(inner),
                ItemRec::When(rec) => {
                    let mut arms = vec![rec.otherwise];
                    for arm in ir
                        .arms
                        .get(rec.arms.0 as usize..rec.arms.1 as usize)
                        .unwrap_or_default()
                    {
                        roots.push(arm.0);
                        arms.push(arm.1);
                    }
                    for arm in arms {
                        if let Some(found) = ir.blocks.get(arm as usize) {
                            items.extend(
                                ir.block_items
                                    .get(found.items.0 as usize..found.items.1 as usize)
                                    .unwrap_or_default(),
                            );
                            items.extend(
                                ir.members
                                    .get(found.members.0 as usize..found.members.1 as usize)
                                    .unwrap_or_default()
                                    .iter()
                                    .filter(|member| **member != NONE),
                            );
                        }
                    }
                }
                ItemRec::Bundle(rec) => items.extend(
                    ir.members
                        .get(rec.members.0 as usize..rec.members.1 as usize)
                        .unwrap_or_default()
                        .iter()
                        .filter(|member| **member != NONE),
                ),
                ItemRec::Paren(_) => {}
            }
        }
    }
    roots.retain(|root| *root != NONE);
    Ok((roots, nested))
}

impl Core<'_> {
    pub fn folds(&self) -> u32 {
        u32::try_from(self.ir.fors.len()).unwrap_or(0)
    }

    pub fn is_fold(&self, owner: u32) -> bool {
        owner >= self.functions() && owner < self.functions() + self.folds()
    }

    pub fn loop_at(&self, owner: u32) -> Option<u32> {
        owner
            .checked_sub(self.functions() + self.folds())
            .filter(|at| (*at as usize) < self.loops.len())
    }

    pub fn loop_owner(&self, item: u32) -> u32 {
        match self.loops.binary_search(&item) {
            Ok(at) => self.functions() + self.folds() + u32::try_from(at).unwrap_or(0),
            Err(_) => NONE,
        }
    }

    pub fn loop_rec(&self, owner: u32) -> Result<ForBlockRec, Internal> {
        let item = self
            .loop_at(owner)
            .and_then(|at| self.loops.get(at as usize))
            .copied()
            .ok_or(Internal::missed("a placement for"))?;
        match *self.ir.item(item)? {
            ItemRec::For(rec) => Ok(rec),
            ItemRec::Placement(_)
            | ItemRec::Wire(_)
            | ItemRec::Bundle(_)
            | ItemRec::When(_)
            | ItemRec::Line(_)
            | ItemRec::Paren(_) => Err(Internal::missed("IR for block")),
        }
    }

    pub fn anchor(&self, owner: u32) -> u32 {
        if owner == NONE || owner < self.functions() {
            return NONE;
        }
        if self.is_fold(owner) {
            return self
                .fold_nodes
                .get((owner - self.functions()) as usize)
                .copied()
                .unwrap_or(NONE);
        }
        self.loop_rec(owner).map_or(NONE, |rec| rec.head.iterable.0)
    }

    pub fn parent_owner(&self, owner: u32) -> Result<u32, Internal> {
        let anchor = self.anchor(owner);
        if anchor == NONE {
            return Ok(NONE);
        }
        Ok(self
            .body_of
            .get(self.local(anchor)?)
            .copied()
            .unwrap_or(NONE))
    }

    pub fn enter_env(&mut self, env: u32) -> (u32, usize, Vec<(u32, u32)>) {
        let mut links = Vec::new();
        let mut at = env;
        while let Some(record) = self.envs.get(at as usize) {
            if record.use_item != NONE {
                links.push((record.use_item, record.inst));
            }
            at = record.parent;
        }
        let saved = (
            self.inst,
            self.own_calls.len(),
            std::mem::replace(&mut self.use_links, links),
        );
        let mut chain = Vec::new();
        let mut at = env;
        while let Some(record) = self.envs.get(at as usize) {
            let values = self
                .env_vals
                .get(record.values.0 as usize..record.values.1 as usize)
                .unwrap_or_default()
                .to_vec();
            chain.push((at, record.inst, values));
            at = record.parent;
        }
        self.inst = self
            .envs
            .get(env as usize)
            .map_or(NONE, |record| record.inst);
        self.own_calls.extend(chain.into_iter().rev());
        saved
    }

    pub fn leave_env(&mut self, saved: (u32, usize, Vec<(u32, u32)>)) {
        self.inst = saved.0;
        self.own_calls.truncate(saved.1);
        self.use_links = saved.2;
    }

    fn iterable_span(&self, head: Iteration) -> Result<Span, Internal> {
        let first = self.node_span(head.iterable.0)?;
        if !head.range {
            return Ok(first);
        }
        let last = self.node_span(head.iterable.1)?;
        Ok(Span {
            file: first.file,
            range: first.range.start..last.range.end,
        })
    }

    pub fn loop_head(&mut self, item: u32, env: u32) -> Stage<Option<LoopRun>> {
        let ItemRec::For(rec) = *self.ir.item(item)? else {
            return Err(Internal::missed("IR for block"));
        };
        let saved = self.enter_env(env);
        for root in iterable_roots(rec.head) {
            match self.reads_deferred(root) {
                Ok(false) => {}
                Ok(true) => {
                    self.leave_env(saved);
                    return Ok(Err(Declined(Reason::PatternUse)));
                }
                Err(internal) => {
                    self.leave_env(saved);
                    return Err(internal);
                }
            }
        }
        let outcome = self.loop_head_in(item, rec, env);
        self.leave_env(saved);
        outcome
    }

    fn loop_head_in(&mut self, item: u32, rec: ForBlockRec, env: u32) -> Stage<Option<LoopRun>> {
        let head = rec.head;
        let names = head.names.1.saturating_sub(head.names.0);
        if head.range {
            if names >= 2 {
                let binder = *self.ir.binder(head.names.0 + 1)?;
                let file = self.file_of(head.iterable.0);
                let span = self.span(file, binder.lo, binder.hi);
                let diagnostic = Diagnostic::new(Code::IterableType, span, RANGE_NAMES)
                    .with_help("write one name over a range, as in `for i in 0..4`");
                self.diagnostics.push(diagnostic);
                return Ok(Ok(None));
            }
            let mut bounds = Vec::new();
            for bound in [head.iterable.0, head.iterable.1] {
                match self.range_bound(bound)? {
                    Ok(Some(value)) => bounds.push(value),
                    Ok(None) => return Ok(Ok(None)),
                    Err(declined) => return Ok(Err(declined)),
                }
            }
            if self.family(head.iterable.0).is_some() || self.family(head.iterable.1).is_some() {
                for bound in [head.iterable.0, head.iterable.1] {
                    self.unit_position(bound, "a range bound")?;
                }
                return Ok(Ok(None));
            }
            let (low, high) = match bounds.as_slice() {
                [low, high] => (*low, *high),
                _ => return Err(Internal::missed("two range bounds")),
            };
            let count = usize::try_from((high - low).max(0)).unwrap_or(usize::MAX);
            if count > 0
                && let Some(ty) = self.binder_ty(head.names.0)?
                && let Some(range) = crate::semantics::operators::int_range(ty)
                && (!range.contains(&low) || !range.contains(&(high - 1)))
            {
                let span = self.iterable_span(head)?;
                self.draw(
                    Code::DoesNotFit,
                    span,
                    format!(
                        "this value does not fit `{}`",
                        crate::semantics::operators::ty_name(ty)
                    ),
                );
            }
            return Ok(Ok(Some(LoopRun {
                item,
                items: Value::Int(low),
                count,
                facts: self.annotated_loop_names(rec, vec![Fact::scalar(Leaf::Int)])?,
                next: 0,
                env,
                inst: NONE,
                open: false,
            })));
        }
        let root = head.iterable.0;
        match self.cell_head(root, REACTIVE, false)? {
            Ok(true) => return Ok(Ok(None)),
            Ok(false) => {}
            Err(declined) => return Ok(Err(declined)),
        }
        let fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if fact.node {
            return self.reference_loop(item, rec, env);
        }
        let open = fact.leaf == Leaf::Placeholder
            || (fact.leaf == Leaf::Error && fact.dims.is_empty())
            || fact.dims.first() == Some(&ANY_LENGTH);
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(if open {
                Ok(Some(LoopRun::unrun(item, env)))
            } else {
                Err(Declined(Reason::LoopHead))
            });
        }
        if self.is_reactive(root)? {
            self.draw_at(Code::ReactiveIterable, root, REACTIVE)?;
            return Ok(Ok(None));
        }
        if fact.dims.is_empty() {
            self.draw_at(Code::IterableType, root, ITERABLE)?;
            return Ok(Ok(None));
        }
        let Some(own) = self.own_ty(root)? else {
            return Ok(Ok(None));
        };
        let element_fact = Fact {
            leaf: fact.leaf,
            dims: fact.dims.get(1..).unwrap_or_default().to_vec(),
            fixed: fact.fixed,
            read: fact.read,
            node: false,
            family: None,
        };
        let items_ty = match self.binder_ty(head.names.1 - 1)? {
            Some(annotated) if refusal(&element_fact, annotated) == Refusal::Kept => annotated,
            Some(_) | None => own,
        };
        if self.holds_kept(root)? {
            return Ok(if open {
                Ok(Some(LoopRun::unrun(item, env)))
            } else {
                Err(Declined(Reason::LoopHead))
            });
        }
        let items = match self.evaluate(root, items_ty)? {
            Ok(items) => items,
            Err(declined) => return Ok(Err(declined)),
        };
        if self.outside_slice(root)? {
            return Ok(Ok(Some(LoopRun::unrun(item, env))));
        }
        let count = match &items {
            Value::Array(array) => self.store.len(*array) as usize,
            Value::Error if open => return Ok(Ok(Some(LoopRun::unrun(item, env)))),
            Value::Error => return Ok(Err(Declined(Reason::LoopHead))),
            Value::Int(_)
            | Value::F32(_)
            | Value::F64(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_) => return Ok(Err(Declined(Reason::LoopHead))),
        };
        let element = Fact {
            leaf: fact.leaf,
            dims: fact.dims.get(1..).unwrap_or_default().to_vec(),
            fixed: fact.fixed,
            read: fact.read,
            node: false,
            family: fact.family,
        };
        let facts = if names >= 2 {
            vec![Fact::scalar(Leaf::Int), element]
        } else {
            vec![element]
        };
        let facts = self.annotated_loop_names(rec, facts)?;
        Ok(Ok(Some(LoopRun {
            item,
            items,
            count,
            facts,
            next: 0,
            env,
            inst: NONE,
            open: false,
        })))
    }

    pub fn reaches_cell(&self, root: u32) -> Result<u8, Internal> {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![(root, false)];
        let mut reached = NOT_A_CELL;
        let mut choice = false;
        while let Some((current, through_let)) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            for node in self.deep_postorder(current)? {
                let (op, _, _, _) = self.ir.node(node)?;
                if matches!(op, Op::Field | Op::To | Op::Past | Op::Stepped) {
                    return Ok(THROUGH_A_PATH);
                }
                choice |= matches!(op, Op::If | Op::Match);
                if matches!(op, Op::Call | Op::NamedCall) {
                    if let Some(function) = self.callee(node)? {
                        stack.extend(
                            self.function_roots(function)?
                                .into_iter()
                                .map(|root| (root, through_let)),
                        );
                    }
                    continue;
                }
                if op != Op::Name {
                    continue;
                }
                match self.bound(node)? {
                    DeclRef::Cell(_) => {
                        reached = reached.max(if through_let { THROUGH_A_LET } else { DIRECTLY });
                    }
                    DeclRef::Let(at) => {
                        let top = self
                            .ir
                            .lets
                            .get(at as usize)
                            .is_some_and(|rec| rec.owner == NONE);
                        stack.push((self.let_root(at)?, through_let || top));
                    }
                    DeclRef::Param(position) => {
                        if self
                            .param_fact(node, position)
                            .is_ok_and(|(_, reactive)| reactive)
                        {
                            reached =
                                reached.max(if through_let { THROUGH_A_LET } else { DIRECTLY });
                        }
                    }
                    DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Fn(_)
                    | DeclRef::Pattern(_) => {}
                }
            }
        }
        if choice && reached != NOT_A_CELL {
            return Ok(THROUGH_A_PATH);
        }
        Ok(reached)
    }

    pub fn calls_reach_a_cell(
        &self,
        root: u32,
        reaching: &mut Option<Vec<bool>>,
    ) -> Result<bool, Internal> {
        for node in self.postorder(root)? {
            if !matches!(self.ir.node(node)?.0, Op::Call | Op::NamedCall) {
                continue;
            }
            let Some(function) = self.callee(node)? else {
                continue;
            };
            if reaching.is_none() {
                *reaching = Some(self.cell_reaching_functions()?);
            }
            if reaching
                .as_ref()
                .and_then(|table| table.get(function as usize))
                .copied()
                .unwrap_or(false)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn cell_reaching_functions(&self) -> Result<Vec<bool>, Internal> {
        let functions = self.functions() as usize;
        let owners = functions + self.ir.lets.len();
        let mut reached = vec![false; owners];
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for owner in 0..owners {
            let roots = match owner.checked_sub(functions) {
                None => self.function_roots(u32::try_from(owner).unwrap_or(u32::MAX))?,
                Some(at) => vec![self.let_root(u32::try_from(at).unwrap_or(u32::MAX))?],
            };
            let reader = u32::try_from(owner).unwrap_or(u32::MAX);
            for body in roots {
                for node in self.deep_postorder(body)? {
                    CountedWalk::CoreCellReach.step();
                    let (op, _, _, _) = self.ir.node(node)?;
                    let base = if matches!(op, Op::Field | Op::To | Op::Past | Op::Stepped) {
                        true
                    } else if matches!(op, Op::Call | Op::NamedCall) {
                        if let Some(function) = self.callee(node)? {
                            edges.push((function, reader));
                        }
                        false
                    } else if op == Op::Name {
                        match self.bound(node)? {
                            DeclRef::Cell(_) => true,
                            DeclRef::Let(at) => {
                                let read = functions.saturating_add(at as usize);
                                edges.push((u32::try_from(read).unwrap_or(u32::MAX), reader));
                                false
                            }
                            DeclRef::Param(position) => self
                                .param_fact(node, position)
                                .is_ok_and(|(_, reactive)| reactive),
                            DeclRef::None
                            | DeclRef::Instance(_)
                            | DeclRef::Duplicate
                            | DeclRef::Fn(_)
                            | DeclRef::Pattern(_) => false,
                        }
                    } else {
                        false
                    };
                    if base && let Some(slot) = reached.get_mut(owner) {
                        *slot = true;
                    }
                }
            }
        }
        edges.sort_unstable();
        let mut stack: Vec<u32> = (0..owners)
            .filter(|at| reached.get(*at).copied().unwrap_or(false))
            .filter_map(|at| u32::try_from(at).ok())
            .collect();
        while let Some(read) = stack.pop() {
            let from = edges.partition_point(|(target, _)| *target < read);
            for (target, reader) in edges.get(from..).unwrap_or_default() {
                if *target != read {
                    break;
                }
                CountedWalk::CoreCellReach.step();
                if let Some(slot) = reached.get_mut(*reader as usize)
                    && !*slot
                {
                    *slot = true;
                    stack.push(*reader);
                }
            }
        }
        reached.truncate(functions);
        Ok(reached)
    }

    fn holds_kept(&self, root: u32) -> Result<bool, Internal> {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![root];
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            for node in self.deep_postorder(current)? {
                if self.is_kept(node)? {
                    return Ok(true);
                }
                if self.ir.node(node)?.0 == Op::Name
                    && let DeclRef::Let(at) = self.bound(node)?
                {
                    stack.push(self.let_root(at)?);
                }
            }
        }
        Ok(false)
    }

    fn cell_head(&mut self, root: u32, message: &str, bound: bool) -> Stage<bool> {
        let reached = self.reaches_cell(root)?;
        if reached == THROUGH_A_PATH && self.ir.node(root)?.0 == Op::If {
            return self.choice_head(root, message, bound);
        }
        match reached {
            NOT_A_CELL => Ok(Ok(false)),
            THROUGH_A_LET => Ok(Ok(true)),
            DIRECTLY => {
                let (op, a, b, _) = self.ir.node(root)?;
                let literal = |node: u32| {
                    self.ir
                        .node(node)
                        .is_ok_and(|(op, _, _, _)| op == Op::Array)
                };
                let array = if bound || matches!(op, Op::Array | Op::Slice) {
                    Some(true)
                } else if op == Op::Name
                    && let DeclRef::Cell(at) = self.bound(root)?
                {
                    self.cell_shape(at)?
                } else if op == Op::Add && (literal(a) || literal(b)) {
                    Some(true)
                } else {
                    None
                };
                match array {
                    Some(true) => self.draw_at(Code::ReactiveIterable, root, message)?,
                    Some(false) => self.draw_at(Code::IterableType, root, ITERABLE)?,
                    None => return Ok(Err(Declined(Reason::LoopHead))),
                }
                Ok(Ok(true))
            }
            _ => Ok(Err(Declined(Reason::LoopHead))),
        }
    }

    fn cell_shape(&self, at: u32) -> Result<Option<bool>, Internal> {
        let rec = *self
            .ir
            .cells
            .get(at as usize)
            .ok_or(Internal::missed("IR cell"))?;
        let annotation = self.ir.binder(rec.binder)?.annotation;
        let node = if annotation != NONE {
            annotation
        } else if rec.init != NONE {
            rec.init
        } else {
            rec.rule
        };
        if node == NONE {
            return Ok(None);
        }
        let op = self.ir.node(node)?.0;
        Ok(if matches!(op, Op::TyArray | Op::Array) {
            Some(true)
        } else if matches!(op, Op::TyScalar | Op::Int | Op::Float | Op::Bool) {
            Some(false)
        } else {
            None
        })
    }

    fn range_bound(&mut self, bound: u32) -> Stage<Option<i128>> {
        match self.cell_head(bound, REACTIVE_BOUND, true)? {
            Ok(true) => return Ok(Ok(None)),
            Ok(false) => {}
            Err(declined) => return Ok(Err(declined)),
        }
        let fact = match self.ensure_typed(bound)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if fact.node {
            return Ok(Err(Declined(Reason::LoopHead)));
        }
        if !fact.dims.is_empty() && fact.leaf != Leaf::Error {
            self.draw_at(
                Code::RangeBound,
                bound,
                "a range bound is an integer, not an array",
            )?;
            return Ok(Ok(None));
        }
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Ok(None));
        }
        if self.is_reactive(bound)? {
            self.draw_at(Code::ReactiveIterable, bound, REACTIVE_BOUND)?;
            return Ok(Ok(None));
        }
        if !matches!(fact.leaf, Leaf::Int) {
            self.draw_at(Code::RangeBound, bound, "a range bound is an integer")?;
            return self.own_check(bound).map(|checked| checked.map(|()| None));
        }
        let value = match self.evaluate(bound, EvalTy::Exact)? {
            Ok(Value::Int(value)) => value,
            Ok(_) => return Ok(Ok(None)),
            Err(declined) => return Ok(Err(declined)),
        };
        if i64::try_from(value).is_err() {
            self.draw_at(
                Code::RangeBound,
                bound,
                format!("the range bound {value} does not fit `i64`"),
            )?;
            return Ok(Ok(None));
        }
        Ok(Ok(Some(value)))
    }

    pub fn loop_step(&mut self, item: u32) -> Result<bool, Internal> {
        if self.stop.is_some() {
            return Ok(false);
        }
        let ItemRec::For(rec) = *self.ir.item(item)? else {
            return Err(Internal::missed("IR for block"));
        };
        let span = self.iterable_span(rec.head)?;
        Ok(self.expansion_step(span))
    }

    pub fn expansion_step(&mut self, span: Span) -> bool {
        self.fold_steps += 1;
        let bound = self.limits.expansion_steps;
        if self.fold_steps > bound {
            let message = format!(
                "the expansion runs more than {} `for` iterations and pattern uses",
                grouped(bound)
            );
            self.stop = Some((
                self.diagnostics.len(),
                Diagnostic::new(Code::ExpansionLimit, span, message),
            ));
            return false;
        }
        if self.fold_steps - 1 == self.limits.expansion_steps_warning {
            let counted = format!(
                "the expansion runs more than {} `for` iterations and pattern uses",
                grouped(self.limits.expansion_steps_warning)
            );
            self.warn_span(span, "expansion_steps", counted, bound);
        }
        true
    }

    pub fn loop_env(&mut self, run: &mut LoopRun) -> Result<u32, Internal> {
        let owner = self.loop_owner(run.item);
        let position = run.next;
        let element = match &run.items {
            Value::Int(low) => {
                i128::try_from(position).map_or(Value::Error, |at| Value::Int(low + at))
            }
            Value::Array(array) => self.store.element(*array, position).unwrap_or(Value::Error),
            Value::F32(_)
            | Value::F64(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Error => Value::Error,
        };
        let element = match (run.facts.last().and_then(|fact| fact.fixed), &element) {
            (Some(ty), Value::Int(_)) if is_fixed_float(ty) => convert_to(element, ty),
            _ => element,
        };
        let referenced = match &element {
            Value::Node(row)
                if run
                    .facts
                    .last()
                    .is_some_and(|fact| fact.node && fact.dims.is_empty()) =>
            {
                Some(*row)
            }
            _ => None,
        };
        let start = u32::try_from(self.env_vals.len()).unwrap_or(NONE);
        if run.facts.len() >= 2 {
            self.env_vals
                .push(i128::try_from(position).map_or(Value::Error, Value::Int));
        }
        self.env_vals.push(element);
        let end = u32::try_from(self.env_vals.len()).unwrap_or(NONE);
        let inst = match referenced {
            Some(row) if self.reads_paths_through(owner)? => self.ref_inst(owner, run, row)?,
            Some(_) | None => {
                if run.inst == NONE {
                    run.inst = self.loop_inst(owner, run)?;
                }
                run.inst
            }
        };
        CountedWalk::CoreLoopIteration.step();
        self.envs.push(Env {
            parent: run.env,
            inst,
            values: (start, end),
            at: u32::try_from(position).unwrap_or(NONE),
            use_item: NONE,
        });
        Ok(u32::try_from(self.envs.len() - 1).unwrap_or(NONE))
    }

    fn loop_inst(&mut self, owner: u32, run: &LoopRun) -> Result<u32, Internal> {
        let parent = self
            .envs
            .get(run.env as usize)
            .map_or(NONE, |record| record.inst);
        let reactive_args = vec![false; run.facts.len()];
        let key = (owner, parent, run.facts.clone(), reactive_args.clone());
        Ok(match self.body_index.get(&key).copied() {
            Some(inst) => inst,
            None => {
                CountedWalk::CoreLoopBody.step();
                self.bodies.push(BodyInstance {
                    function: owner,
                    parent,
                    args: run.facts.clone(),
                    reactive_args,
                    diagnostics: Vec::new(),
                    values: Vec::new(),
                });
                let inst = u32::try_from(self.bodies.len() - 1).unwrap_or(NONE);
                self.body_index.insert(key, inst);
                inst
            }
        })
    }
}

impl Core<'_> {
    pub fn unrun_names(&mut self, item: u32, file: u32) -> Stage<()> {
        let ItemRec::For(rec) = *self.ir.item(item)? else {
            return Err(Internal::missed("IR for block"));
        };
        self.unrun_block(rec.body, file)
    }

    pub fn unrun_block(&mut self, body: u32, file: u32) -> Stage<()> {
        let mut blocks = vec![body];
        let mut items = Vec::new();
        while let Some(block) = blocks.pop() {
            let Some(found) = self.ir.blocks.get(block as usize) else {
                continue;
            };
            items.extend(
                self.ir
                    .block_items
                    .get(found.items.0 as usize..found.items.1 as usize)
                    .unwrap_or_default()
                    .iter()
                    .rev(),
            );
            items.extend(member_items(self.ir, found.members));
            while let Some(child) = items.pop() {
                match *self.ir.item(child)? {
                    ItemRec::Placement(placement) => {
                        if let Err(declined) = self.unrun_placement(placement, file)? {
                            return Ok(Err(declined));
                        }
                        blocks.push(placement.body);
                    }
                    ItemRec::For(inner) => blocks.push(inner.body),
                    ItemRec::Line(members) => items.extend(
                        self.ir
                            .members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default()
                            .iter()
                            .rev(),
                    ),
                    ItemRec::Wire(wire) if self.bare_wire(wire)? => {}
                    ItemRec::Wire(wire) if self.plain_wire(wire)? => self.unrun_wire(wire, file)?,
                    ItemRec::When(_) => items.extend(self.when_items(child).into_iter().rev()),
                    ItemRec::Bundle(_) => items.extend(self.bundle_items(child)),
                    ItemRec::Paren(inner) if inner != NONE => items.push(inner),
                    ItemRec::Wire(_) | ItemRec::Paren(_) => {
                        return Ok(Err(Declined(Reason::BodyItem)));
                    }
                }
            }
        }
        Ok(Ok(()))
    }

    fn unrun_placement(&mut self, placement: PlacementRec, file: u32) -> Stage<()> {
        if let Some(pattern) = self.used_pattern(placement.kind) {
            return self.unrun_use(placement, pattern, file);
        }
        let Some(kind_name) = self
            .ir
            .name_text(placement.kind)
            .filter(|_| self.ir.named(placement.kind))
        else {
            return Ok(Ok(()));
        };
        let duplicated = self
            .ir
            .name(placement.kind)
            .is_some_and(|entry| self.decl.get(entry.sym.0 as usize) == Some(&DeclRef::Duplicate));
        let found = self
            .vocabulary
            .node_kind_named(kind_name)
            .filter(|_| !duplicated)
            .and_then(|kind| self.vocabulary.node_kind(kind));
        let Some(decl) = found else {
            if placement.inline || self.vocabulary.wire_kind_named(kind_name).is_some() {
                return Ok(Err(Declined(Reason::BodyItem)));
            }
            self.unknown_kind_drawn(file, placement.kind)?;
            return Ok(Ok(()));
        };
        let kind_text = decl.name.clone();
        let names: Vec<String> = decl.attrs.iter().map(|attr| attr.name.clone()).collect();
        for at in placement.attrs.0..placement.attrs.1 {
            let entry = *self
                .ir
                .attr_entries
                .get(at as usize)
                .ok_or(Internal::missed("IR attribute"))?;
            let name = *self
                .ir
                .name(entry.name)
                .ok_or(Internal::missed("IR name"))?;
            let text = self.ir.symbols.text(name.sym).to_string();
            if !names.contains(&text) {
                let span = self.span(file, name.lo, name.hi);
                self.draw(
                    Code::UnknownAttribute,
                    span,
                    format!("`{kind_text}` has no attribute `{text}`"),
                );
            }
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    pub fn loop_element(
        &mut self,
        node: u32,
        owner: u32,
        position: u32,
        ty: EvalTy,
        stored: &CoreValue,
    ) -> Stage<CoreValue> {
        let Some(at) = self.loop_at(owner) else {
            if self.pattern_at(owner).is_some() {
                return self.use_element(owner, position, ty, stored);
            }
            return Ok(Ok(stored.clone()));
        };
        let head = self.loop_rec(owner)?.head;
        let names = head.names.1.saturating_sub(head.names.0);
        let class = super::values::class(ty);
        if let Value::Int(value) = stored
            && (head.range || position + 1 != names)
            && matches!(class, 1 | 2)
            && self.param_fact(node, position)?.0.fixed.is_none()
        {
            let float = super::values::evaluated_in(ty);
            let iterable = if head.range {
                head.iterable
            } else {
                (head.iterable.0, NONE)
            };
            return Ok(Ok(self.loop_float(*value, float, iterable)?));
        }
        if head.range || position + 1 != names || class == 3 {
            return Ok(Ok(stored.clone()));
        }
        let fact = self.param_fact(node, position)?.0;
        if fact.fixed.is_some() || !matches!(fact.leaf, Leaf::Int | Leaf::Float) {
            return Ok(Ok(stored.clone()));
        }
        let bodies = &self.bodies;
        let Some(env) = self
            .own_calls
            .iter()
            .rev()
            .find(|(_, inst, _)| {
                bodies
                    .get(*inst as usize)
                    .is_some_and(|body| body.function == owner)
            })
            .map(|(env, _, _)| *env)
        else {
            return Ok(Ok(stored.clone()));
        };
        let Some(record) = self.envs.get(env as usize).copied() else {
            return Ok(Ok(stored.clone()));
        };
        let element = if class == 0 {
            stored.clone()
        } else {
            let item = self
                .loops
                .get(at as usize)
                .copied()
                .ok_or(Internal::missed("a placement for"))?;
            let key = (item, record.parent, class);
            let items = match self.loop_items.get(&key) {
                Some(items) => items.clone(),
                None => {
                    let saved = self.enter_env(record.parent);
                    let chain = std::mem::take(&mut self.eval_chain);
                    let outcome = self.evaluate(head.iterable.0, ty);
                    self.eval_chain = chain;
                    self.leave_env(saved);
                    let items = match outcome? {
                        Ok(items) => items,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    self.loop_items.insert(key, items.clone());
                    items
                }
            };
            match items {
                Value::Array(array) => self
                    .store
                    .element(array, record.at as usize)
                    .unwrap_or(Value::Error),
                Value::Int(_)
                | Value::F32(_)
                | Value::F64(_)
                | Value::Bool(_)
                | Value::Enum(..)
                | Value::Str(_)
                | Value::Node(_)
                | Value::Error => Value::Error,
            }
        };
        if !element.is_error() && !matches!(element, Value::Array(_)) {
            self.loop_reads
                .entry((env, node, class))
                .or_insert_with(|| element.clone());
        }
        Ok(Ok(element))
    }
}

impl Core<'_> {
    pub fn unknown_body(&mut self, body: u32, file: u32) -> Stage<()> {
        let mut blocks = vec![body];
        let mut items = Vec::new();
        while let Some(block) = blocks.pop() {
            let Some(found) = self.ir.blocks.get(block as usize) else {
                continue;
            };
            if found.members.0 != found.members.1 || found.lets.0 != found.lets.1 {
                return Ok(Err(Declined(Reason::BodyItem)));
            }
            items.extend(
                self.ir
                    .block_items
                    .get(found.items.0 as usize..found.items.1 as usize)
                    .unwrap_or_default()
                    .iter()
                    .rev(),
            );
            items.extend(member_items(self.ir, found.members));
            while let Some(child) = items.pop() {
                match *self.ir.item(child)? {
                    ItemRec::Placement(placement) => {
                        if placement.has_as || placement.inline {
                            return Ok(Err(Declined(Reason::BodyItem)));
                        }
                        if let Err(declined) = self.unknown_body_kind(placement, file)? {
                            return Ok(Err(declined));
                        }
                        blocks.push(placement.body);
                    }
                    ItemRec::For(inner) => blocks.push(inner.body),
                    ItemRec::Line(members) => items.extend(
                        self.ir
                            .members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default()
                            .iter()
                            .rev(),
                    ),
                    ItemRec::When(_) => items.extend(self.when_items(child).into_iter().rev()),
                    ItemRec::Bundle(_) => items.extend(self.bundle_items(child)),
                    ItemRec::Paren(inner) if inner != NONE => items.push(inner),
                    ItemRec::Wire(_) | ItemRec::Paren(_) => {
                        return Ok(Err(Declined(Reason::BodyItem)));
                    }
                }
            }
        }
        Ok(Ok(()))
    }

    fn unknown_body_kind(&mut self, placement: PlacementRec, file: u32) -> Stage<()> {
        if self.used_pattern(placement.kind).is_some() {
            return Ok(Ok(()));
        }
        let Some(kind_name) = self
            .ir
            .name_text(placement.kind)
            .filter(|_| self.ir.named(placement.kind))
        else {
            return Ok(Ok(()));
        };
        if self.vocabulary.node_kind_named(kind_name).is_some() {
            return Ok(Ok(()));
        }
        if self.vocabulary.wire_kind_named(kind_name).is_some()
            || self.vocabulary_clash(kind_name).is_some()
            || self.vocabulary.native_named(kind_name).is_some()
        {
            return Ok(Err(Declined(Reason::BodyItem)));
        }
        let entry = *self
            .ir
            .name(placement.kind)
            .ok_or(Internal::missed("IR name"))?;
        let declared = self
            .decl
            .get(entry.sym.0 as usize)
            .is_some_and(|decl| *decl != DeclRef::None);
        if declared || self.skipped_top.binary_search(&entry.sym.0).is_ok() {
            return Ok(Ok(()));
        }
        let text = self.ir.symbols.text(entry.sym).to_string();
        let span = self.name_span(file, placement.kind)?;
        self.draw(
            Code::UnknownKind,
            span,
            format!("`{text}` is not a node kind or a pattern"),
        );
        Ok(Ok(()))
    }
}

impl Core<'_> {
    pub fn open_body(&mut self, item: u32, parent: u32) -> Result<(), Internal> {
        let ItemRec::For(rec) = *self.ir.item(item)? else {
            return Err(Internal::missed("IR for block"));
        };
        let mut kinds = Vec::new();
        let mut unknown = false;
        let mut blocks = vec![(rec.body, true)];
        while let Some((block, direct)) = blocks.pop() {
            let Some(found) = self.ir.blocks.get(block as usize) else {
                continue;
            };
            let mut items: Vec<u32> = self
                .ir
                .block_items
                .get(found.items.0 as usize..found.items.1 as usize)
                .unwrap_or_default()
                .to_vec();
            items.extend(member_items(self.ir, found.members));
            while let Some(child) = items.pop() {
                match *self.ir.item(child)? {
                    ItemRec::Placement(placement) => {
                        let kind = self
                            .ir
                            .name_text(placement.kind)
                            .filter(|_| self.ir.named(placement.kind))
                            .and_then(|name| self.vocabulary.node_kind_named(name));
                        match kind {
                            Some(kind) => {
                                if direct {
                                    kinds.push(kind);
                                }
                                if self
                                    .vocabulary
                                    .node_kind(kind)
                                    .is_some_and(|decl| decl.entry)
                                {
                                    unknown = true;
                                }
                            }
                            None => unknown = true,
                        }
                        blocks.push((placement.body, false));
                    }
                    ItemRec::For(inner) => blocks.push((inner.body, direct)),
                    ItemRec::Line(members) => items.extend(
                        self.ir
                            .members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default(),
                    ),
                    ItemRec::When(_) => items.extend(self.when_items(child)),
                    ItemRec::Bundle(_) => items.extend(self.bundle_items(child)),
                    ItemRec::Paren(inner) if inner != NONE => items.push(inner),
                    ItemRec::Wire(_) | ItemRec::Paren(_) => unknown = true,
                }
            }
        }
        if unknown {
            self.unknown_children.insert(parent);
            self.unknown_kind = true;
        }
        self.open_children.entry(parent).or_default().extend(kinds);
        Ok(())
    }
}

impl Core<'_> {
    pub fn named_in_loops(&mut self) -> Result<(), Internal> {
        for at in 0..self.loops.len() {
            let item = *self
                .loops
                .get(at)
                .ok_or(Internal::missed("a placement for"))?;
            let ItemRec::For(rec) = *self.ir.item(item)? else {
                return Err(Internal::missed("IR for block"));
            };
            let file = self.file_of(rec.head.iterable.0);
            let Some(block) = self.ir.blocks.get(rec.body as usize).copied() else {
                continue;
            };
            let mut items: Vec<u32> = self
                .ir
                .block_items
                .get(block.items.0 as usize..block.items.1 as usize)
                .unwrap_or_default()
                .iter()
                .rev()
                .copied()
                .collect();
            while let Some(child) = items.pop() {
                let name = match *self.ir.item(child)? {
                    ItemRec::Placement(placement) if placement.has_as => placement.as_name,
                    ItemRec::Wire(wire) => wire.as_name,
                    ItemRec::Line(members) => {
                        items.extend(
                            self.ir
                                .members
                                .get(members.0 as usize..members.1 as usize)
                                .unwrap_or_default()
                                .iter()
                                .rev(),
                        );
                        continue;
                    }
                    ItemRec::When(_) => {
                        items.extend(self.when_items(child).into_iter().rev());
                        continue;
                    }
                    ItemRec::Bundle(_) => {
                        items.extend(self.bundle_items(child).into_iter().rev());
                        continue;
                    }
                    ItemRec::Paren(inner) if inner != NONE => {
                        items.push(inner);
                        continue;
                    }
                    ItemRec::Placement(_) | ItemRec::For(_) | ItemRec::Paren(_) => continue,
                };
                if !self.ir.named(name) {
                    continue;
                }
                let span = self.name_span(file, name)?;
                self.draw(
                    Code::NamedInFor,
                    span,
                    "placements in a `for` body are anonymous",
                );
            }
            self.loop_name_clashes(rec, file)?;
        }
        Ok(())
    }
}

impl Core<'_> {
    fn loop_name_clashes(&mut self, rec: ForBlockRec, file: u32) -> Result<(), Internal> {
        let mut names = Vec::new();
        for binder in rec.head.names.0..rec.head.names.1 {
            let at = *self.ir.binder(binder)?;
            let sym = self
                .ir
                .name(at.name)
                .ok_or(Internal::missed("IR name"))?
                .sym;
            names.push((sym, self.span(file, at.lo, at.hi)));
        }
        let mut blocks = vec![(rec.body, false)];
        while let Some((block, nested)) = blocks.pop() {
            let Some(found) = self.ir.blocks.get(block as usize).copied() else {
                continue;
            };
            let mut items: Vec<u32> = self
                .ir
                .block_items
                .get(found.items.0 as usize..found.items.1 as usize)
                .unwrap_or_default()
                .to_vec();
            items.extend(member_items(self.ir, found.members));
            while let Some(child) = items.pop() {
                match *self.ir.item(child)? {
                    ItemRec::Placement(placement) => {
                        if nested && placement.has_as && self.ir.named(placement.as_name) {
                            let entry = *self
                                .ir
                                .name(placement.as_name)
                                .ok_or(Internal::missed("IR name"))?;
                            if let Some((_, repeated)) =
                                names.iter().find(|(sym, _)| *sym == entry.sym).cloned()
                            {
                                let text = self.ir.symbols.text(entry.sym).to_string();
                                let span = self.name_span(file, placement.as_name)?;
                                self.draw_related(
                                    Code::ChildNameClash,
                                    span,
                                    vec![(repeated, "the name it repeats".to_string())],
                                    format!(
                                        "`{text}` names a pattern parameter, a pattern `let` or a `for` variable visible here"
                                    ),
                                );
                            }
                        }
                        blocks.push((placement.body, true));
                    }
                    ItemRec::For(inner) => blocks.push((inner.body, nested)),
                    ItemRec::Line(members) => items.extend(
                        self.ir
                            .members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default(),
                    ),
                    ItemRec::When(_) => items.extend(self.when_items(child)),
                    ItemRec::Bundle(_) => items.extend(self.bundle_items(child)),
                    ItemRec::Paren(inner) if inner != NONE => items.push(inner),
                    ItemRec::Wire(_) | ItemRec::Paren(_) => {}
                }
            }
        }
        Ok(())
    }
}

impl Core<'_> {
    fn annotated_loop_names(
        &mut self,
        rec: ForBlockRec,
        mut facts: Vec<Fact>,
    ) -> Result<Vec<Fact>, Internal> {
        let head = rec.head;
        for (binder, fact) in (head.names.0..head.names.1).zip(facts.iter_mut()) {
            let Some(ty) = self.binder_ty(binder)? else {
                continue;
            };
            let summary = match (refusal(fact, ty), head.range) {
                (Refusal::Shape, _) => Some(ELEMENT_SHAPE),
                (Refusal::Type, false) => Some(ELEMENT_TYPE),
                (Refusal::Type, true) => Some(RANGE_TYPE),
                (Refusal::Kept | Refusal::Silent, _) => None,
            };
            if matches!(ty, EvalTy::Enum(_)) && summary.is_some() {
                let mut span = self.node_span(head.iterable.0)?;
                if head.range {
                    span.range.end = self.node_span(head.iterable.1)?.range.end;
                }
                self.draw(Code::TypeMismatch, span, super::annotation::TYPE);
                *fact = Fact::ERROR;
                continue;
            }
            if let Some(summary) = summary {
                let at = *self.ir.binder(binder)?;
                let span = self.span(self.file_of(head.iterable.0), at.lo, at.hi);
                self.draw(Code::TypeMismatch, span, summary);
            }
            *fact = annotated_fact(ty);
        }
        Ok(facts)
    }
}

impl Core<'_> {
    fn bare_wire(&self, wire: WireRec) -> Result<bool, Internal> {
        let known = self
            .ir
            .name_text(wire.kind)
            .filter(|_| self.ir.named(wire.kind))
            .is_some_and(|name| self.vocabulary.wire_kind_named(name).is_some());
        Ok(known
            && wire.attrs.0 == wire.attrs.1
            && wire.default == NONE
            && wire.gate == NONE
            && self.ir.node(wire.destination)?.0 == Op::Name
            && matches!(self.bound(wire.destination)?, DeclRef::Instance(_)))
    }
}

impl Core<'_> {
    fn outside_slice(&self, root: u32) -> Result<bool, Internal> {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            let (op, a, b, _) = self.ir.node(node)?;
            if op == Op::Slice {
                for bound in self.bounds(node)? {
                    if bound != NONE && self.outside_bound(bound)? {
                        return Ok(true);
                    }
                }
                stack.push(a);
            } else if op == Op::Add {
                stack.extend([a, b]);
            } else if op == Op::Name
                && let DeclRef::Let(at) = self.bound(node)?
            {
                stack.push(self.let_root(at)?);
            }
        }
        Ok(false)
    }

    fn outside_bound(&self, bound: u32) -> Result<bool, Internal> {
        let span = self.node_span(bound)?;
        Ok(self
            .diagnostics
            .iter()
            .any(|drawn| drawn.code == Code::IndexOutOfRange && drawn.span == span))
    }
}

impl Core<'_> {
    fn reference_loop(&mut self, item: u32, rec: ForBlockRec, env: u32) -> Stage<Option<LoopRun>> {
        let head = rec.head;
        let root = head.iterable.0;
        let fact = self.fact(root)?;
        if fact.dims.is_empty()
            || self.is_kept(root)?
            || self.binder_ty(head.names.1 - 1)?.is_some()
        {
            return Ok(Err(Declined(Reason::LoopHead)));
        }
        let items = match self.evaluate(root, EvalTy::Exact)? {
            Ok(items) => items,
            Err(declined) => return Ok(Err(declined)),
        };
        let Value::Array(array) = items else {
            return Ok(Err(Declined(Reason::LoopHead)));
        };
        let count = self.store.len(array) as usize;
        for at in 0..count {
            if !matches!(
                self.store.element(array, at),
                Some(Value::Node(_) | Value::Array(_))
            ) {
                return Ok(Err(Declined(Reason::LoopHead)));
            }
        }
        let element = Fact::node_reference(fact.dims.get(1..).unwrap_or_default().to_vec());
        let names = head.names.1.saturating_sub(head.names.0);
        let facts = if names >= 2 {
            vec![Fact::scalar(Leaf::Int), element]
        } else {
            vec![element]
        };
        let facts = self.annotated_loop_names(rec, facts)?;
        Ok(Ok(Some(LoopRun {
            item,
            items,
            count,
            facts,
            next: 0,
            env,
            inst: NONE,
            open: false,
        })))
    }
}

impl Core<'_> {
    fn choice_head(&mut self, root: u32, message: &str, bound: bool) -> Stage<bool> {
        let (_, condition, then, otherwise) = self.ir.node(root)?;
        let arms = [self.reaches_cell(then)?, self.reaches_cell(otherwise)?];
        let condition_reach = self.reaches_cell(condition)?;
        if condition_reach == NOT_A_CELL {
            let test = match self.ensure_typed(condition)? {
                Ok(test) => test,
                Err(declined) => return Ok(Err(declined)),
            };
            if test.leaf != Leaf::Bool || !test.dims.is_empty() || self.is_kept(condition)? {
                return Ok(Err(Declined(Reason::LoopHead)));
            }
            let (reached, other) = match self.evaluate(condition, EvalTy::Bool)? {
                Ok(Value::Bool(true)) => (arms[0], arms[1]),
                Ok(Value::Bool(false)) => (arms[1], arms[0]),
                Ok(_) => return Ok(Err(Declined(Reason::LoopHead))),
                Err(declined) => return Ok(Err(declined)),
            };
            return Ok(match reached {
                NOT_A_CELL if other == NOT_A_CELL => Ok(false),
                THROUGH_A_LET if other != DIRECTLY => Ok(true),
                _ => Err(Declined(Reason::LoopHead)),
            });
        }
        if condition_reach != DIRECTLY || arms != [NOT_A_CELL; 2] {
            return Ok(Err(Declined(Reason::LoopHead)));
        }
        let fact = match self.ensure_typed(then)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        let other = match self.ensure_typed(otherwise)? {
            Ok(other) => other,
            Err(declined) => return Ok(Err(declined)),
        };
        if other.leaf != fact.leaf || other.dims.len() != fact.dims.len() || other.node {
            return Ok(Err(Declined(Reason::LoopHead)));
        }
        if fact.node || matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Err(Declined(Reason::LoopHead)));
        }
        if bound || !fact.dims.is_empty() {
            self.draw_at(Code::ReactiveIterable, root, message)?;
        } else {
            self.draw_at(Code::IterableType, root, ITERABLE)?;
        }
        Ok(Ok(true))
    }
}

impl Core<'_> {
    fn plain_wire(&self, wire: WireRec) -> Result<bool, Internal> {
        let bare = WireRec {
            attrs: (0, 0),
            default: NONE,
            ..wire
        };
        if wire.attrs_cut || !self.bare_wire(bare)? {
            return Ok(false);
        }
        let mut values = vec![wire.default];
        for at in wire.attrs.0..wire.attrs.1 {
            let entry = *self
                .ir
                .attr_entries
                .get(at as usize)
                .ok_or(Internal::missed("IR attribute"))?;
            let AttrForm::Value(value) = entry.form else {
                return Ok(false);
            };
            values.push(value);
        }
        for value in values.into_iter().filter(|value| *value != NONE) {
            for node in self.deep_postorder(value)? {
                if matches!(self.ir.node(node)?.0, Op::To | Op::Field) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    fn unrun_wire(&mut self, wire: WireRec, file: u32) -> Result<(), Internal> {
        let Some(decl) = self
            .ir
            .name_text(wire.kind)
            .and_then(|name| self.vocabulary.wire_kind_named(name))
            .and_then(|kind| self.vocabulary.wire_kind(kind))
        else {
            return Err(Internal::missed("wire kind"));
        };
        let kind_text = decl.name.clone();
        let names: Vec<String> = decl.attrs.iter().map(|attr| attr.name.clone()).collect();
        for at in wire.attrs.0..wire.attrs.1 {
            let entry = *self
                .ir
                .attr_entries
                .get(at as usize)
                .ok_or(Internal::missed("IR attribute"))?;
            let name = *self
                .ir
                .name(entry.name)
                .ok_or(Internal::missed("IR name"))?;
            let text = self.ir.symbols.text(name.sym).to_string();
            if !names.contains(&text) {
                let span = self.span(file, name.lo, name.hi);
                self.draw(
                    Code::UnknownAttribute,
                    span,
                    format!("`{kind_text}` has no attribute `{text}`"),
                );
            }
        }
        Ok(())
    }
}

impl Core<'_> {
    fn reads_paths_through(&mut self, owner: u32) -> Result<bool, Internal> {
        if self.path_owners.is_none() {
            let mut owners = std::collections::HashSet::new();
            for local in 0..self.ir.op.len() {
                if self.ir.op.get(local) != Some(&Op::Field) {
                    continue;
                }
                let receiver = self.ir.a.get(local).copied().unwrap_or(NONE);
                if self.ir.node(receiver)?.0 != Op::Name
                    || !matches!(self.bound(receiver)?, DeclRef::Param(_))
                {
                    continue;
                }
                let bound = self.binding_owner(receiver)?;
                if self.loop_at(bound).is_some() {
                    owners.insert(bound);
                }
            }
            self.path_owners = Some(owners);
        }
        Ok(self
            .path_owners
            .as_ref()
            .is_some_and(|owners| owners.contains(&owner)))
    }

    fn ref_inst(&mut self, owner: u32, run: &LoopRun, row: u32) -> Result<u32, Internal> {
        let parent = self
            .envs
            .get(run.env as usize)
            .map_or(NONE, |record| record.inst);
        if let Some(inst) = self.ref_insts.get(&(owner, parent, row)).copied() {
            return Ok(inst);
        }
        self.bodies.push(BodyInstance {
            function: owner,
            parent,
            args: run.facts.clone(),
            reactive_args: vec![false; run.facts.len()],
            diagnostics: Vec::new(),
            values: Vec::new(),
        });
        let inst = u32::try_from(self.bodies.len() - 1).unwrap_or(NONE);
        let position = u32::try_from(run.facts.len().saturating_sub(1)).unwrap_or(NONE);
        self.ref_insts.insert((owner, parent, row), inst);
        self.inst_row.insert(inst, (position, row));
        self.owner_insts.entry(owner).or_default().push(inst);
        Ok(inst)
    }
}

pub(crate) fn member_items(ir: &FlatIr, members: (u32, u32)) -> Vec<u32> {
    ir.members
        .get(members.0 as usize..members.1 as usize)
        .unwrap_or_default()
        .iter()
        .copied()
        .filter(|member| *member != NONE)
        .collect()
}
