use super::annotation::{Refusal, SHAPE, TYPE, annotated_fact, refusal};
use super::body_instance::BodyInstance;
use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::Fact;
use super::leaf::Leaf;
use super::need::Need;
use super::reason::Reason;
use super::typing::unify;
use super::width::{combined, is_fixed_float};
use crate::diagnostics::{Code, Diagnostic, for_start_and_body_help};
use crate::semantics::Internal;
use crate::semantics::ir::flat::{ForRec, NONE, Op};
use crate::semantics::operators::{EvalTy, Value, convert_to};
use crate::semantics::reporting::grouped;

pub(crate) const ANY_LENGTH: u32 = u32::MAX;

pub(crate) fn dims_agree(a: &[u32], b: &[u32]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x == y || *x == ANY_LENGTH || *y == ANY_LENGTH)
}

const ROUNDS: u32 = 4;

const ELEMENT_TYPE: &str =
    "the elements do not have the loop name's annotated type; a conversion states the rounding";
const ELEMENT_SHAPE: &str = "the elements' shape differs from the loop name's annotated type";
const RANGE_TYPE: &str = "a range's values do not have the loop name's annotated type";

const START_AND_BODY: &str =
    "the start and the body of a `for` expression hold values of different types";
const LAYOUTS: &str =
    "the start and the body of a `for` expression hold arrays of different shapes";
const ITERABLE: &str = "a `for` iterates over a range or an array";
const RANGE_NAMES: &str =
    "a range binds one name; two names iterate an array, its positions and its elements";

fn faulty(fact: &Fact) -> bool {
    matches!(fact.leaf, Leaf::Placeholder | Leaf::Error)
}

fn variable(fact: &Fact) -> Fact {
    let mut fact = fact.clone();
    if let Some(first) = fact.dims.first_mut() {
        *first = ANY_LENGTH;
    }
    fact
}

fn merged_dims(start: &[u32], body: &[u32]) -> Option<Vec<u32>> {
    match (start.split_first(), body.split_first()) {
        (None, None) => Some(Vec::new()),
        (Some((outer, inner)), Some((_, body_inner))) => {
            if start == body && !start.contains(&ANY_LENGTH) {
                return Some(body.to_vec());
            }
            let empty = *outer == 0 && inner.is_empty();
            let inner = if empty { body_inner } else { inner };
            if body_inner != inner {
                return None;
            }
            let mut dims = vec![ANY_LENGTH];
            dims.extend_from_slice(inner);
            Some(dims)
        }
        (Some(_), None) | (None, Some(_)) => None,
    }
}

impl Core<'_> {
    pub fn fold_rec(&self, node: u32) -> Result<ForRec, Internal> {
        let (_, record, _, _) = self.ir.node(node)?;
        self.ir
            .fors
            .get(record as usize)
            .copied()
            .ok_or(Internal::missed("IR for"))
    }

    pub fn fold_operands(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let rec = self.fold_rec(node)?;
        let mut operands = vec![rec.start, rec.iterable.0];
        if rec.range {
            operands.push(rec.iterable.1);
        }
        Ok(operands)
    }

    pub fn fold_names(&self, node: u32) -> Result<u32, Internal> {
        let rec = self.fold_rec(node)?;
        Ok(rec.names.1 - rec.names.0)
    }

    pub fn in_fold(&self, node: u32) -> Result<bool, Internal> {
        let owner = self.body_of.get(self.local(node)?).copied().unwrap_or(NONE);
        Ok(owner != NONE && owner >= self.functions())
    }

    pub fn function_owner(&self, node: u32) -> Result<u32, Internal> {
        let mut owner = self.body_of.get(self.local(node)?).copied().unwrap_or(NONE);
        while owner != NONE && owner >= self.functions() {
            let fold = self
                .fold_nodes
                .get((owner - self.functions()) as usize)
                .copied()
                .unwrap_or(NONE);
            if fold == NONE {
                return Ok(NONE);
            }
            owner = self.body_of.get(self.local(fold)?).copied().unwrap_or(NONE);
        }
        Ok(owner)
    }

    pub fn enclosing_syms(&self, owner: u32) -> Result<Vec<u32>, Internal> {
        let mut syms = Vec::new();
        let mut owner = owner;
        while owner != NONE && owner >= self.functions() {
            let fold = self
                .fold_nodes
                .get((owner - self.functions()) as usize)
                .copied()
                .unwrap_or(NONE);
            if fold == NONE {
                break;
            }
            owner = self.body_of.get(self.local(fold)?).copied().unwrap_or(NONE);
            if owner != NONE {
                syms.extend(self.scope_of(owner)?.into_iter().map(|(sym, _)| sym));
            }
        }
        Ok(syms)
    }

    pub fn deep_postorder(&self, root: u32) -> Result<Vec<u32>, Internal> {
        let mut nodes = self.postorder(root)?;
        let mut cursor = 0;
        while let Some(node) = nodes.get(cursor).copied() {
            cursor += 1;
            let (op, record, _, _) = self.ir.node(node)?;
            if op == Op::For {
                for fold_root in self.function_roots(self.functions() + record)? {
                    nodes.extend(self.postorder(fold_root)?);
                }
            }
        }
        Ok(nodes)
    }

    fn fold_head(&mut self, node: u32, rec: &ForRec) -> Result<Option<Vec<Fact>>, Internal> {
        let names = rec.names.1 - rec.names.0;
        if rec.range {
            let bounds = [rec.iterable.0, rec.iterable.1];
            if names >= 2 {
                let binder = *self.ir.binder(rec.names.0 + 1)?;
                let span = self.span(self.file_of(node), binder.lo, binder.hi);
                let diagnostic = Diagnostic::new(Code::IterableType, span, RANGE_NAMES)
                    .with_help("write one name over a range, as in `for i in 0..4`");
                let diagnostic = self.chained(diagnostic);
                self.diagnostics.push(diagnostic);
                return Ok(None);
            }
            for bound in bounds {
                let fact = self.fact(bound)?;
                if fact.leaf == Leaf::Placeholder {
                    return Ok(None);
                }
                if !fact.dims.is_empty() && fact.leaf != Leaf::Error {
                    self.draw_at(
                        Code::RangeBound,
                        bound,
                        "a range bound is an integer, not an array",
                    )?;
                    return Ok(None);
                }
            }
            for bound in bounds {
                let fact = self.fact(bound)?;
                if faulty(&fact) {
                    return Ok(None);
                }
                if !matches!(fact.leaf, Leaf::Int | Leaf::Empty) {
                    self.draw_at(Code::RangeBound, bound, "a range bound is an integer")?;
                    return Ok(None);
                }
                if let Ok(Value::Int(value)) = self.evaluate(bound, EvalTy::Exact)?
                    && i64::try_from(value).is_err()
                {
                    self.draw_at(
                        Code::RangeBound,
                        bound,
                        format!("the range bound {value} does not fit `i64`"),
                    )?;
                    return Ok(None);
                }
            }
            return self
                .annotated_items(node, rec, vec![Fact::scalar(Leaf::Int)])
                .map(Some);
        }
        let values = rec.iterable.0;
        let fact = self.fact(values)?;
        if faulty(&fact) {
            return Ok(None);
        }
        if fact.dims.is_empty() {
            self.draw_at(Code::IterableType, values, ITERABLE)?;
            return Ok(None);
        }
        let element = Fact {
            leaf: fact.leaf,
            dims: fact.dims.get(1..).unwrap_or_default().to_vec(),
            fixed: fact.fixed,
            read: fact.read,
        };
        let items = if names >= 2 {
            vec![Fact::scalar(Leaf::Int), element]
        } else {
            vec![element]
        };
        self.annotated_items(node, rec, items).map(Some)
    }

    fn annotated_items(
        &mut self,
        node: u32,
        rec: &ForRec,
        mut items: Vec<Fact>,
    ) -> Result<Vec<Fact>, Internal> {
        for (binder, item) in (rec.names.0..rec.names.1).zip(items.iter_mut()) {
            let Some(ty) = self.binder_ty(binder)? else {
                continue;
            };
            let summary = match (refusal(item, ty), rec.range) {
                (Refusal::Shape, _) => Some(ELEMENT_SHAPE),
                (Refusal::Type, false) => Some(ELEMENT_TYPE),
                (Refusal::Type, true) => Some(RANGE_TYPE),
                (Refusal::Kept | Refusal::Silent, _) => None,
            };
            if let Some(summary) = summary {
                let at = *self.ir.binder(binder)?;
                let span = self.span(self.file_of(node), at.lo, at.hi);
                let diagnostic = Diagnostic::new(Code::TypeMismatch, span, summary);
                let diagnostic = self.chained(diagnostic);
                self.diagnostics.push(diagnostic);
            }
            *item = annotated_fact(ty);
        }
        Ok(items)
    }

    pub fn accumulator_ty(&self, rec: &ForRec) -> Result<Option<EvalTy>, Internal> {
        self.binder_ty(rec.accumulator)
    }

    pub fn item_ty(&self, rec: &ForRec, position: usize) -> Result<Option<EvalTy>, Internal> {
        let binder = rec.names.0 + u32::try_from(position).unwrap_or(u32::MAX);
        if binder >= rec.names.1 {
            return Ok(None);
        }
        self.binder_ty(binder)
    }

    fn annotated_fold(
        &mut self,
        node: u32,
        rec: &ForRec,
        items: Vec<Fact>,
        ty: EvalTy,
    ) -> Stage<Fact> {
        let (_, record, _, _) = self.ir.node(node)?;
        let start = self.fact(rec.start)?;
        match refusal(&start, ty) {
            Refusal::Shape => self.draw_at(Code::TypeMismatch, rec.start, SHAPE)?,
            Refusal::Type => self.draw_at(Code::TypeMismatch, rec.start, TYPE)?,
            Refusal::Kept | Refusal::Silent => {}
        }
        let owner = self.functions() + record;
        let mut args = items;
        args.push(annotated_fact(ty));
        let body = match self.fold_instance(node, owner, args)? {
            Ok(body) => body,
            Err(declined) => return Ok(Err(declined)),
        };
        let outer = std::mem::replace(&mut self.inst, body);
        let found = self.fact(rec.body);
        let reactive = self.is_reactive(rec.body);
        self.inst = outer;
        if reactive? {
            return Ok(Err(Declined(Reason::ReactiveFor)));
        }
        let found = found?;
        let key = self.inst_key(node)?;
        self.call_body.insert((key, node), body);
        self.replay_fold(node, body)?;
        match refusal(&found, ty) {
            Refusal::Shape => self.draw_at(Code::TypeMismatch, rec.body, SHAPE)?,
            Refusal::Type => self.draw_at(Code::TypeMismatch, rec.body, TYPE)?,
            Refusal::Kept | Refusal::Silent => {}
        }
        Ok(Ok(annotated_fact(ty)))
    }

    pub fn annotated_fold_refused(&mut self, node: u32) -> Result<bool, Internal> {
        let rec = self.fold_rec(node)?;
        let Some(ty) = self.accumulator_ty(&rec)? else {
            return Ok(false);
        };
        if refusal(&self.fact(rec.start)?, ty) != Refusal::Kept {
            return Ok(true);
        }
        let Some(body) = self.body_of_call(node) else {
            return Ok(true);
        };
        let outer = std::mem::replace(&mut self.inst, body);
        let found = self.fact(rec.body);
        self.inst = outer;
        Ok(!matches!(
            refusal(&found?, ty),
            Refusal::Kept | Refusal::Silent
        ))
    }

    fn fold_instance(&mut self, node: u32, owner: u32, args: Vec<Fact>) -> Stage<u32> {
        let reactive_args = vec![false; args.len()];
        let reaches_out = self
            .reaches_out
            .get((owner - self.functions()) as usize)
            .copied()
            .unwrap_or(false);
        let parent = if reaches_out {
            self.inst_key(node)?
        } else {
            NONE
        };
        let key = (owner, parent, args.clone(), reactive_args.clone());
        if let Some(at) = self.body_index.get(&key).copied() {
            return Ok(Ok(at));
        }
        self.bodies.push(BodyInstance {
            function: owner,
            parent,
            args,
            reactive_args,
            diagnostics: Vec::new(),
        });
        let at = u32::try_from(self.bodies.len() - 1).unwrap_or(NONE);
        self.body_index.insert(key, at);
        self.pending(Need::Body(at))
    }

    pub fn for_fact(&mut self, node: u32) -> Stage<Fact> {
        let (_, record, _, _) = self.ir.node(node)?;
        let rec = self.fold_rec(node)?;
        for operand in self.fold_operands(node)? {
            if self.is_reactive(operand)? {
                return Ok(Err(Declined(Reason::ReactiveFor)));
            }
        }
        let Some(items) = self.fold_head(node, &rec)? else {
            return Ok(Ok(Fact::PLACEHOLDER));
        };
        if let Some(ty) = self.accumulator_ty(&rec)? {
            return self.annotated_fold(node, &rec, items, ty);
        }
        let start = self.fact(rec.start)?;
        if faulty(&start) {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        let owner = self.functions() + record;
        let mut acc = variable(&start);
        let mut round = 0;
        let (body, body_fact) = loop {
            let mut args = items.clone();
            args.push(acc.clone());
            let body = match self.fold_instance(node, owner, args)? {
                Ok(body) => body,
                Err(declined) => return Ok(Err(declined)),
            };
            let outer = std::mem::replace(&mut self.inst, body);
            let found = self.fact(rec.body);
            let reactive = self.is_reactive(rec.body);
            self.inst = outer;
            if reactive? {
                return Ok(Err(Declined(Reason::ReactiveFor)));
            }
            let found = found?;
            if faulty(&found) {
                if round == 0 && !found.dims.is_empty() && acc != start {
                    acc = start.clone();
                    round = 1;
                    continue;
                }
                break (body, found);
            }
            let Some(dims) = merged_dims(&start.dims, &found.dims) else {
                self.replay_fold(node, body)?;
                let span = self.node_span(node)?;
                let diagnostic = Diagnostic::new(Code::TypeMismatch, span, LAYOUTS)
                    .with_help(for_start_and_body_help());
                let diagnostic = self.chained(diagnostic);
                self.diagnostics.push(diagnostic);
                return Ok(Ok(Fact::PLACEHOLDER));
            };
            let (leaf, fixed) = match unify(start.leaf, found.leaf) {
                Some(leaf) => (
                    leaf,
                    combined(start.fixed, found.fixed, leaf == Leaf::Float),
                ),
                None => (start.leaf, start.fixed),
            };
            let next = Fact {
                leaf,
                dims,
                fixed,
                read: None,
            };
            let settled =
                (next.leaf == acc.leaf || acc.leaf == Leaf::Empty) && next.dims == acc.dims;
            if settled || round + 1 >= ROUNDS {
                break (body, found);
            }
            acc = next;
            round += 1;
        };
        let key = self.inst_key(node)?;
        self.call_body.insert((key, node), body);
        self.replay_fold(node, body)?;
        if faulty(&body_fact) {
            return Ok(Ok(if body_fact.leaf == Leaf::Placeholder {
                Fact::PLACEHOLDER
            } else {
                Fact::ERROR
            }));
        }
        let Some(leaf) = unify(start.leaf, body_fact.leaf) else {
            let span = self.node_span(node)?;
            let diagnostic = Diagnostic::new(Code::TypeMismatch, span, START_AND_BODY)
                .with_help(for_start_and_body_help());
            let diagnostic = self.chained(diagnostic);
            self.diagnostics.push(diagnostic);
            return Ok(Ok(Fact::ERROR));
        };
        let fixed = combined(start.fixed, body_fact.fixed, leaf == Leaf::Float);
        Ok(Ok(Fact {
            leaf,
            dims: acc.dims,
            fixed,
            read: None,
        }))
    }

    fn replay_fold(&mut self, _node: u32, body: u32) -> Result<(), Internal> {
        let replayed: Vec<_> = self
            .bodies
            .get(body as usize)
            .map(|instance| instance.diagnostics.clone())
            .unwrap_or_default();
        for diagnostic in replayed {
            if self.eval_chain.is_empty() && self.inst == NONE {
                self.diagnostics.push(diagnostic);
            } else {
                self.push_one_per_chain(diagnostic, self.inst != NONE);
            }
        }
        Ok(())
    }

    pub fn fold_step(&mut self, node: u32) -> Stage<bool> {
        if self.stop.is_some() {
            return Ok(Ok(false));
        }
        self.fold_steps += 1;
        let bound = self.limits.expansion_steps;
        if self.fold_steps > bound {
            let message = format!(
                "the expansion runs more than {} `for` iterations and pattern uses",
                grouped(bound)
            );
            self.stop_at(Code::ExpansionLimit, node, message)?;
            return Ok(Ok(false));
        }
        if self.fold_steps - 1 == self.limits.expansion_steps_warning {
            let counted = format!(
                "the expansion runs more than {} `for` iterations and pattern uses",
                grouped(self.limits.expansion_steps_warning)
            );
            self.warn_at(node, "expansion_steps", counted, bound)?;
        }
        Ok(Ok(true))
    }

    pub fn fold_item(
        &mut self,
        fold: u32,
        items: &CoreValue,
        position: usize,
    ) -> Result<CoreValue, Internal> {
        let item = match items {
            Value::Array(array) => self.store.element(*array, position).unwrap_or(Value::Error),
            Value::Int(start) => i128::try_from(position)
                .map_or(Value::Error, |position| Value::Int(start + position)),
            Value::F32(_)
            | Value::F64(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Str(_)
            | Value::Node(_)
            | Value::Error => Value::Error,
        };
        let rec = self.fold_rec(fold)?;
        let element = usize::from(rec.names.1 - rec.names.0 >= 2);
        Ok(match (self.item_ty(&rec, element)?, &item) {
            (Some(ty), Value::Int(_)) if is_fixed_float(ty) => convert_to(item, ty),
            _ => item,
        })
    }
}

impl Core<'_> {
    pub fn holds_name_placeholder(&self, node: u32) -> Result<bool, Internal> {
        for at in self.postorder(node)? {
            let (op, _, _, _) = self.ir.node(at)?;
            if op != Op::Name {
                continue;
            }
            match self.bound(at)? {
                DeclRef::None | DeclRef::Duplicate => return Ok(true),
                DeclRef::Param(_) if self.fact(at)?.leaf == Leaf::Placeholder => return Ok(true),
                DeclRef::Let(_)
                | DeclRef::Cell(_)
                | DeclRef::Instance(_)
                | DeclRef::Param(_)
                | DeclRef::Fn(_) => {}
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    pub fn stop_at(&mut self, code: Code, node: u32, message: String) -> Result<(), Internal> {
        if self.stop.is_some() {
            return Ok(());
        }
        let span = self.node_span(node)?;
        self.unmemoized();
        let diagnostic = Diagnostic::new(code, span, message);
        self.stop = Some((self.diagnostics.len(), diagnostic));
        Ok(())
    }

    pub fn warn_at(
        &mut self,
        node: u32,
        key: &str,
        counted: String,
        bound: u64,
    ) -> Result<(), Internal> {
        if self.stop.is_some() || !self.warned.insert(node) {
            return Ok(());
        }
        self.unmemoized();
        let span = self.node_span(node)?;
        let diagnostic = Diagnostic::new(
            Code::LimitWarning,
            span,
            format!(
                "{counted}, the warning value of the limit of {}",
                grouped(bound)
            ),
        )
        .with_help(format!(
            "reduce the work, or raise `{key}_warning` in the package's `options` or with `--limit {key}_warning=<value>`"
        ));
        let diagnostic = self.chained(diagnostic);
        self.diagnostics.push(diagnostic);
        Ok(())
    }
}
