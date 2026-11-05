use super::body_instance::BodyInstance;
use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::env::Env;
use super::fact::Fact;
use super::leaf::Leaf;
use super::reason::Reason;
use crate::diagnostics::{Code, Related};
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{ABSENT, AttrForm, ItemRec, NONE, Op, PlacementRec};
use crate::semantics::operators::EvalTy;

impl Core<'_> {
    pub fn used_pattern(&self, kind: u32) -> Option<u32> {
        if !self.ir.named(kind) {
            return None;
        }
        let entry = self.ir.name(kind)?;
        match self.decl.get(entry.sym.0 as usize) {
            Some(DeclRef::Pattern(at)) => Some(*at),
            Some(
                DeclRef::None
                | DeclRef::Let(_)
                | DeclRef::Cell(_)
                | DeclRef::Instance(_)
                | DeclRef::Duplicate
                | DeclRef::Param(_)
                | DeclRef::Fn(_),
            )
            | None => None,
        }
    }

    pub fn unused_patterns(&mut self) -> Stage<()> {
        for pattern in 0..u32::try_from(self.ir.patterns.len()).unwrap_or(0) {
            if self.patterns_used.contains(&pattern) {
                continue;
            }
            if self.names_in_pattern(pattern)? {
                return Ok(Err(Declined(Reason::PatternUse)));
            }
            if self.own_reads.contains(&pattern) {
                self.written_paths(pattern)?;
            }
            let body = self.pattern_parts(pattern)?.1;
            let file = self.pattern_file(pattern);
            if let Err(declined) = self.unrun_block(body, file)? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }
}

impl Core<'_> {
    pub fn pattern_declarations(&mut self) -> Result<(), Internal> {
        for pattern in 0..u32::try_from(self.ir.patterns.len()).unwrap_or(0) {
            let params = self.pattern_parts(pattern)?.0;
            let file = self.pattern_file(pattern);
            let mut seen = false;
            for binder in params.0..params.1 {
                let rec = *self.ir.binder(binder)?;
                if !rec.is_default {
                    continue;
                }
                if seen {
                    let span = self.span(file, rec.lo, rec.hi);
                    self.draw(
                        Code::DuplicateDefaultParameter,
                        span,
                        "a pattern has at most one parameter marked `default`",
                    );
                }
                seen = true;
            }
        }
        Ok(())
    }
}

impl Core<'_> {
    pub fn use_env(
        &mut self,
        item: u32,
        pattern: u32,
        (file, env): (u32, u32),
        top: bool,
    ) -> Stage<Option<u32>> {
        let placement = self.placement(item)?;
        if (!top && (self.ir.named(placement.as_name) || placement.has_as))
            || self.names_in_pattern(pattern)?
            || placement.body != NONE
            || placement.inline
        {
            return Ok(Err(Declined(Reason::PatternUse)));
        }
        self.patterns_used.insert(pattern);
        CountedWalk::CorePatternUse.step();
        let span = self.span(file, placement.lo, placement.hi);
        if !self.expansion_step(span.clone()) {
            return Ok(Ok(None));
        }
        let bound = match self.use_sources(placement, pattern, (file, env), span.clone())? {
            Ok(bound) => bound,
            Err(declined) => return Ok(Err(declined)),
        };
        let params = self.pattern_parts(pattern)?.0;
        let mut defaults = Vec::new();
        for binder in params.0..params.1 {
            defaults.push(self.ir.binder(binder)?.default);
        }
        let mut facts = Vec::new();
        let mut reactive = Vec::new();
        let mut faults = Vec::new();
        let mut temps = Vec::new();
        let mut late: Vec<(u32, bool)> = Vec::new();
        let start = u32::try_from(self.env_vals.len()).unwrap_or(NONE);
        for (position, (root, caller)) in bound.clone().into_iter().enumerate() {
            reactive.push(false);
            if root == NONE && caller != NONE {
                facts.push(Fact::scalar(Leaf::Bool));
                self.env_vals.push(CoreValue::Bool(caller == 1));
                continue;
            }
            if root == NONE {
                facts.push(Fact::PLACEHOLDER);
                self.env_vals.push(CoreValue::Error);
                continue;
            }
            let defaulted = defaults.get(position) == Some(&root);
            if defaulted && let Some(cells) = self.late_reads(root, &late)? {
                facts.push(Fact::PLACEHOLDER);
                self.env_vals.push(CoreValue::Error);
                if let Some(slot) = reactive.last_mut() {
                    *slot = cells;
                }
                late.push((u32::try_from(position).unwrap_or(NONE), cells));
                continue;
            }
            if !defaulted {
                let saved = self.enter_env(caller);
                let deferred = self.deferred_arg(root);
                self.leave_env(saved);
                if let Some(cells) = deferred? {
                    facts.push(Fact::PLACEHOLDER);
                    self.env_vals.push(CoreValue::Error);
                    if let Some(slot) = reactive.last_mut() {
                        *slot = cells;
                    }
                    late.push((u32::try_from(position).unwrap_or(NONE), cells));
                    continue;
                }
            }
            let value = if defaulted {
                let temp = self.default_inst(pattern, env, (&facts, &reactive), root)?;
                if temp != NONE {
                    temps.push((position, temp));
                }
                let drawn = self.diagnostics.len();
                let saved = self.enter_env(NONE);
                self.enter_default(temp, start);
                let typed = self.ensure_typed(root);
                self.leave_env(saved);
                let own = self.diagnostics.split_off(drawn);
                let held = self.default_faults.entry((root, temp)).or_default();
                held.extend(own);
                faults.extend(held.iter().cloned());
                if let Err(declined) = typed? {
                    return Ok(Err(declined));
                }
                self.use_spans.insert(item, span.clone());
                let saved = self.enter_env(env);
                self.use_links.insert(0, (item, NONE));
                self.enter_default(temp, start);
                let value = self.argument(root);
                self.leave_env(saved);
                value
            } else {
                let saved = self.enter_env(caller);
                let value = self.argument(root);
                self.leave_env(saved);
                value
            };
            match value? {
                Ok((fact, value, cells)) => {
                    facts.push(fact);
                    self.env_vals.push(value);
                    if let Some(slot) = reactive.last_mut() {
                        *slot = cells;
                    }
                }
                Err(declined) => return Ok(Err(declined)),
            }
        }
        let made = self.use_made(
            item,
            pattern,
            (env, start, late.is_empty()),
            (facts, reactive),
            (span, bound),
        )?;
        if let Ok(Some(made)) = made
            && !late.is_empty()
        {
            self.defer_use(made, &late);
        }
        if let Ok(Some(made)) = made {
            for (position, temp) in temps {
                self.default_insts
                    .insert((made, u32::try_from(position).unwrap_or(NONE)), temp);
            }
        }
        if let Ok(Some(made)) = made
            && !faults.is_empty()
            && let Some(record) = self.envs.get(made as usize).copied()
        {
            let held = self.use_faults.entry(record.inst).or_default();
            for fault in faults {
                if !held.contains(&fault) {
                    held.push(fault);
                }
            }
        }
        Ok(made)
    }

    pub fn use_sources(
        &mut self,
        placement: PlacementRec,
        pattern: u32,
        (file, env): (u32, u32),
        span: symphonyscript_core::Span,
    ) -> Stage<Vec<(u32, u32)>> {
        let params = self.pattern_parts(pattern)?.0;
        let count = (params.1 - params.0) as usize;
        let mut sources: Vec<Option<(u32, u32)>> = vec![None; count];
        if placement.default != NONE {
            let mut position = None;
            for (at, binder) in (params.0..params.1).enumerate() {
                if self.ir.binder(binder)?.is_default {
                    position = Some(at);
                    break;
                }
            }
            match position.and_then(|at| sources.get_mut(at)) {
                Some(slot) => *slot = Some((placement.default, env)),
                None => self.draw_at(
                    Code::NoDefaultParameter,
                    placement.default,
                    "`/expr` supplies the parameter marked `default`, and this pattern has none",
                )?,
            }
        }
        for at in placement.attrs.0..placement.attrs.1 {
            let entry = *self
                .ir
                .attr_entries
                .get(at as usize)
                .ok_or(Internal::missed("IR attribute"))?;
            let value = match entry.form {
                AttrForm::Value(value) => (value, env),
                AttrForm::Set(set) => (NONE, u32::from(set)),
            };
            let name = *self
                .ir
                .name(entry.name)
                .ok_or(Internal::missed("IR name"))?;
            let text = self.ir.symbols.text(name.sym).to_string();
            let name_span = self.span(file, name.lo, name.hi);
            let mut position = None;
            for (at, binder) in (params.0..params.1).enumerate() {
                let param = self.ir.binder(binder)?.name;
                if self.ir.named(param) && self.ir.name_text(param) == Some(text.as_str()) {
                    position = Some(at);
                    break;
                }
            }
            match position.and_then(|at| sources.get_mut(at)) {
                None => self.draw(
                    Code::UnknownParameter,
                    name_span,
                    format!("the pattern has no parameter `{text}`"),
                ),
                Some(Some(_)) => self.draw(
                    Code::DuplicateArgument,
                    name_span,
                    format!("the parameter `{text}` is supplied twice"),
                ),
                Some(slot @ None) => *slot = Some(value),
            }
        }
        let mut bound = Vec::new();
        for (binder, source) in (params.0..params.1).zip(sources) {
            let rec = *self.ir.binder(binder)?;
            match source {
                Some(source) => bound.push(source),
                None if rec.default != NONE => bound.push((rec.default, NONE)),
                None => {
                    let text = self.ir.name_text(rec.name).unwrap_or_default().to_string();
                    self.draw(
                        Code::MissingArgument,
                        span.clone(),
                        format!("the parameter `{text}` has no default and no argument"),
                    );
                    bound.push((NONE, NONE));
                }
            }
        }
        Ok(Ok(bound))
    }

    fn default_inst(
        &mut self,
        pattern: u32,
        env: u32,
        (facts, reactive): (&[Fact], &[bool]),
        root: u32,
    ) -> Result<u32, Internal> {
        if !self.default_nodes.contains_key(&root) {
            return Ok(NONE);
        }
        let owner = self.patterns_start() + pattern;
        let parent = self
            .envs
            .get(env as usize)
            .map_or(NONE, |record| record.inst);
        let reactive = reactive.get(..facts.len()).unwrap_or_default().to_vec();
        let key = (owner, parent, facts.to_vec(), reactive.clone());
        if let Some(inst) = self.body_index.get(&key).copied() {
            return Ok(inst);
        }
        self.bodies.push(BodyInstance {
            function: owner,
            parent,
            args: facts.to_vec(),
            reactive_args: reactive,
            diagnostics: Vec::new(),
        });
        let inst = u32::try_from(self.bodies.len() - 1).unwrap_or(NONE);
        self.body_index.insert(key, inst);
        Ok(inst)
    }

    fn enter_default(&mut self, temp: u32, start: u32) {
        if temp == NONE {
            return;
        }
        let values = self
            .env_vals
            .get(start as usize..)
            .unwrap_or_default()
            .to_vec();
        self.own_calls.push((NONE, temp, values));
        self.inst = temp;
    }

    fn use_made(
        &mut self,
        item: u32,
        pattern: u32,
        (env, start, indexed): (u32, u32, bool),
        (facts, reactive_args): (Vec<Fact>, Vec<bool>),
        (span, bound): (symphonyscript_core::Span, Vec<(u32, u32)>),
    ) -> Stage<Option<u32>> {
        let end = u32::try_from(self.env_vals.len()).unwrap_or(NONE);
        let parent = self
            .envs
            .get(env as usize)
            .map_or(NONE, |record| record.inst);
        let owner = self.patterns_start() + pattern;
        let key = (owner, parent, facts.clone(), reactive_args.clone());
        let found = if indexed {
            self.body_index.get(&key).copied()
        } else {
            None
        };
        let inst = match found {
            Some(inst) => inst,
            None => {
                CountedWalk::CorePatternBody.step();
                self.bodies.push(BodyInstance {
                    function: owner,
                    parent,
                    args: facts,
                    reactive_args,
                    diagnostics: Vec::new(),
                });
                let inst = u32::try_from(self.bodies.len() - 1).unwrap_or(NONE);
                if indexed {
                    self.body_index.insert(key, inst);
                }
                inst
            }
        };
        self.use_spans.insert(item, span);
        self.envs.push(Env {
            parent: env,
            inst,
            values: (start, end),
            at: 0,
            use_item: item,
        });
        let made = u32::try_from(self.envs.len() - 1).unwrap_or(NONE);
        self.use_args.insert(made, bound);
        Ok(Ok(Some(made)))
    }

    fn argument(&mut self, root: u32) -> Stage<(Fact, CoreValue, bool)> {
        for node in self.postorder(root)? {
            if matches!(self.ir.node(node)?.0, Op::Field | Op::To) {
                return Ok(Err(Declined(Reason::PatternUse)));
            }
        }
        if self.reaches_cell(root)? != 0 {
            return Ok(Err(Declined(Reason::PatternUse)));
        }
        let fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if fact.node || self.is_reactive(root)? {
            return Ok(Err(Declined(Reason::PatternUse)));
        }
        self.argument_value(root, fact)
    }

    fn late_argument(&mut self, root: u32) -> Stage<(Fact, CoreValue, bool)> {
        let fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if fact.node {
            return Ok(Err(Declined(Reason::PatternUse)));
        }
        if self.is_reactive(root)? {
            return Ok(Ok((fact, CoreValue::Error, true)));
        }
        self.argument_value(root, fact)
    }

    fn deferred_arg(&self, root: u32) -> Result<Option<bool>, Internal> {
        let mut path = false;
        for node in self.postorder(root)? {
            path |= matches!(self.ir.node(node)?.0, Op::Field | Op::To);
        }
        if !path && self.reaches_cell(root)? != 0 {
            return Ok(Some(true));
        }
        if path || self.reads_deferred(root)? {
            return Ok(Some(false));
        }
        Ok(None)
    }

    pub fn reads_deferred(&self, root: u32) -> Result<bool, Internal> {
        if self.deferred_params.is_empty() {
            return Ok(false);
        }
        for node in self.deep_postorder(root)? {
            if self.ir.node(node)?.0 != Op::Name {
                continue;
            }
            if let DeclRef::Param(position) = self.bound(node)? {
                let inst = self.owner_inst(self.binding_owner(node)?);
                if self.deferred_params.contains(&(inst, position)) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn late_reads(&self, root: u32, late: &[(u32, bool)]) -> Result<Option<bool>, Internal> {
        if late.is_empty() || !self.default_nodes.contains_key(&root) {
            return Ok(None);
        }
        let mut found = None;
        for node in self.postorder(root)? {
            if self.ir.node(node)?.0 == Op::Name
                && let DeclRef::Param(position) = self.bound(node)?
                && let Some((_, cells)) = late.iter().find(|(at, _)| *at == position)
            {
                found = Some(found.unwrap_or(false) || *cells);
            }
        }
        Ok(found)
    }

    fn defer_use(&mut self, made: u32, late: &[(u32, bool)]) {
        let Some(record) = self.envs.get(made as usize).copied() else {
            return;
        };
        for (position, cells) in late {
            if !cells {
                self.deferred_params.insert((record.inst, *position));
            }
        }
        self.deferred_uses
            .push((made, late.iter().map(|(position, _)| *position).collect()));
    }

    pub fn key_deferred_uses(&mut self) -> Stage<()> {
        let deferred = std::mem::take(&mut self.deferred_uses);
        for (env, positions) in deferred {
            let record = *self
                .envs
                .get(env as usize)
                .ok_or(Internal::missed("use env"))?;
            let args = self.use_args.get(&env).cloned().unwrap_or_default();
            for position in positions {
                let Some((root, caller)) = args.get(position as usize).copied() else {
                    continue;
                };
                let outcome = if caller == NONE && self.default_nodes.contains_key(&root) {
                    self.late_default(env, record, position, root)?
                } else {
                    let saved = self.enter_env(caller);
                    let outcome = self.late_argument(root);
                    self.leave_env(saved);
                    outcome?
                };
                let (fact, value, cells) = match outcome {
                    Ok(found) => found,
                    Err(declined) => return Ok(Err(declined)),
                };
                if let Some(body) = self.bodies.get_mut(record.inst as usize) {
                    if let Some(slot) = body.args.get_mut(position as usize) {
                        *slot = fact;
                    }
                    if let Some(slot) = body.reactive_args.get_mut(position as usize) {
                        *slot = cells;
                    }
                }
                if let Some(slot) = self.env_vals.get_mut((record.values.0 + position) as usize) {
                    *slot = value;
                }
            }
            let body = self
                .bodies
                .get(record.inst as usize)
                .ok_or(Internal::missed("body instance"))?;
            let parent = self
                .typed_as
                .get(&body.parent)
                .copied()
                .unwrap_or(body.parent);
            let key = (
                body.function,
                parent,
                body.args.clone(),
                body.reactive_args.clone(),
            );
            match self.body_index.get(&key).copied() {
                Some(shared) => {
                    self.typed_as.insert(record.inst, shared);
                }
                None => {
                    self.body_index.insert(key, record.inst);
                }
            }
        }
        Ok(Ok(()))
    }

    fn late_default(
        &mut self,
        env: u32,
        record: Env,
        position: u32,
        root: u32,
    ) -> Stage<(Fact, CoreValue, bool)> {
        let body = self
            .bodies
            .get(record.inst as usize)
            .ok_or(Internal::missed("body instance"))?;
        let pattern = self
            .pattern_at(body.function)
            .ok_or(Internal::missed("a use's pattern"))?;
        let facts = body
            .args
            .get(..position as usize)
            .unwrap_or_default()
            .to_vec();
        let reactive = body.reactive_args.clone();
        let temp = self.default_inst(pattern, record.parent, (&facts, &reactive), root)?;
        self.default_insts.insert((env, position), temp);
        let values = (record.values.0, record.values.1);
        let drawn = self.diagnostics.len();
        let saved = self.enter_env(NONE);
        self.enter_values(temp, values);
        let typed = self.ensure_typed(root);
        self.leave_env(saved);
        let own = self.diagnostics.split_off(drawn);
        let held = self.default_faults.entry((root, temp)).or_default();
        held.extend(own);
        let faults = held.clone();
        let kept = self.use_faults.entry(record.inst).or_default();
        for fault in faults {
            if !kept.contains(&fault) {
                kept.push(fault);
            }
        }
        if let Err(declined) = typed? {
            return Ok(Err(declined));
        }
        let saved = self.enter_env(record.parent);
        self.use_links.insert(0, (record.use_item, NONE));
        self.enter_values(temp, values);
        let value = self.late_argument(root);
        self.leave_env(saved);
        value
    }

    fn enter_values(&mut self, temp: u32, (start, end): (u32, u32)) {
        if temp == NONE {
            return;
        }
        let values = self
            .env_vals
            .get(start as usize..end as usize)
            .unwrap_or_default()
            .to_vec();
        self.own_calls.push((NONE, temp, values));
        self.inst = temp;
    }

    fn argument_value(&mut self, root: u32, fact: Fact) -> Stage<(Fact, CoreValue, bool)> {
        if fact.leaf == Leaf::Placeholder {
            return Ok(Ok((Fact::PLACEHOLDER, CoreValue::Error, false)));
        }
        if fact.leaf == Leaf::Error {
            return Ok(Ok((fact, CoreValue::Error, false)));
        }
        let Some(own) = self.own_ty(root)? else {
            return Ok(Err(Declined(Reason::PatternUse)));
        };
        let value = match self.evaluate(root, own)? {
            Ok(value) => value,
            Err(declined) => return Ok(Err(declined)),
        };
        if value.is_error() {
            return Ok(Ok((fact, value, false)));
        }
        if fact.fixed.is_none() && fact.leaf != Leaf::Float && !self.integral_value(&value) {
            return Ok(Err(Declined(Reason::PatternUse)));
        }
        if fact.fixed.is_none() && fact.leaf == Leaf::Int {
            let float = match self.evaluate(root, EvalTy::F64)? {
                Ok(float) => float,
                Err(declined) => return Ok(Err(declined)),
            };
            if !self.same_number(&value, &float) {
                return Ok(Err(Declined(Reason::PatternUse)));
            }
        }
        Ok(Ok((fact, value, false)))
    }

    fn integral_value(&mut self, value: &CoreValue) -> bool {
        let mut values = vec![value.clone()];
        while let Some(value) = values.pop() {
            match value {
                CoreValue::Array(array) => {
                    for at in 0..self.store.len(array) as usize {
                        values.push(self.store.element(array, at).unwrap_or(CoreValue::Error));
                    }
                }
                CoreValue::Int(_) | CoreValue::Bool(_) => {}
                CoreValue::F32(_)
                | CoreValue::F64(_)
                | CoreValue::Enum(..)
                | CoreValue::Str(_)
                | CoreValue::Node(_)
                | CoreValue::Error => return false,
            }
        }
        true
    }

    pub fn use_element(
        &mut self,
        owner: u32,
        position: u32,
        ty: EvalTy,
        stored: &CoreValue,
    ) -> Stage<CoreValue> {
        let class = super::values::class(ty);
        if matches!(class, 0 | 3) {
            return Ok(Ok(stored.clone()));
        }
        let bodies = &self.bodies;
        let Some((env, inst)) = self
            .own_calls
            .iter()
            .rev()
            .find(|(_, inst, _)| {
                bodies
                    .get(*inst as usize)
                    .is_some_and(|body| body.function == owner)
            })
            .map(|(env, inst, _)| (*env, *inst))
        else {
            return Ok(Ok(stored.clone()));
        };
        let fact = self
            .bodies
            .get(inst as usize)
            .and_then(|body| body.args.get(position as usize))
            .cloned()
            .ok_or(Internal::missed("argument fact"))?;
        let cells = self
            .bodies
            .get(inst as usize)
            .and_then(|body| body.reactive_args.get(position as usize))
            .copied()
            .unwrap_or(false);
        if !cells && (fact.fixed.is_some() || !matches!(fact.leaf, Leaf::Int | Leaf::Float)) {
            return Ok(Ok(stored.clone()));
        }
        if let Some(value) = self.use_values.get(&(env, position, class)) {
            return Ok(Ok(value.clone()));
        }
        let Some((root, caller)) = self
            .use_args
            .get(&env)
            .and_then(|args| args.get(position as usize))
            .copied()
        else {
            return Ok(Ok(stored.clone()));
        };
        let defaulted = match self.pattern_at(owner) {
            Some(pattern) => {
                let params = self.pattern_parts(pattern)?.0;
                self.ir.binder(params.0 + position)?.default == root
            }
            None => false,
        };
        let saved = self.enter_env(if defaulted { env } else { caller });
        if defaulted {
            self.inst = self
                .default_insts
                .get(&(env, position))
                .copied()
                .unwrap_or(NONE);
        }
        let chain = std::mem::take(&mut self.eval_chain);
        let outcome = self.evaluate(root, ty);
        self.eval_chain = chain;
        self.leave_env(saved);
        let value = match outcome? {
            Ok(value) => value,
            Err(declined) => return Ok(Err(declined)),
        };
        self.use_values
            .insert((env, position, class), value.clone());
        Ok(Ok(value))
    }

    pub fn replay_use_faults(&mut self) {
        for env in 0..self.envs.len() {
            let Some(record) = self.envs.get(env).copied() else {
                continue;
            };
            if record.use_item == NONE {
                continue;
            }
            let shared = self
                .typed_as
                .get(&record.inst)
                .copied()
                .unwrap_or(record.inst);
            let mut faults = self
                .use_faults
                .get(&record.inst)
                .cloned()
                .unwrap_or_default();
            if shared != record.inst {
                for fault in self.use_faults.get(&shared).cloned().unwrap_or_default() {
                    if !faults.contains(&fault) {
                        faults.push(fault);
                    }
                }
            }
            if faults.is_empty() {
                continue;
            }
            let saved = self.enter_env(u32::try_from(env).unwrap_or(NONE));
            for fault in faults {
                let mut related = fault.related.clone();
                for (item, _) in &self.use_links {
                    if let Some(span) = self.use_spans.get(item) {
                        related.push(Related {
                            span: span.clone(),
                            label: "in this use of the pattern".to_string(),
                        });
                    }
                }
                if self.use_links.len() < 2 {
                    self.diagnostics.push(fault.with_related(related));
                } else {
                    self.push_one_per_chain(fault.with_related(related), false);
                }
            }
            self.leave_env(saved);
        }
    }
}

impl Core<'_> {
    pub fn names_in_pattern(&self, pattern: u32) -> Result<bool, Internal> {
        Ok(self
            .named_patterns
            .get(pattern as usize)
            .copied()
            .unwrap_or(true))
    }

    fn pattern_items_declined(&self, pattern: u32) -> Result<bool, Internal> {
        let mut blocks = vec![self.pattern_parts(pattern)?.1];
        let mut items = Vec::new();
        while let Some(block) = blocks.pop() {
            let Some(found) = self.ir.blocks.get(block as usize) else {
                continue;
            };
            items.extend(
                self.ir
                    .block_items
                    .get(found.items.0 as usize..found.items.1 as usize)
                    .unwrap_or_default(),
            );
            while let Some(item) = items.pop() {
                match *self.ir.item(item)? {
                    ItemRec::Placement(placement) => {
                        if placement.has_as && !self.ir.named(placement.as_name) {
                            return Ok(true);
                        }
                        blocks.push(placement.body);
                    }
                    ItemRec::Wire(wire) => {
                        if wire.as_name != NONE {
                            return Ok(true);
                        }
                    }
                    ItemRec::For(rec) => blocks.push(rec.body),
                    ItemRec::Line(members) => items.extend(
                        self.ir
                            .members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default(),
                    ),
                    ItemRec::Bundle(_) | ItemRec::When(_) | ItemRec::Paren(_) => {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    fn patterns_in_body(&self, pattern: u32) -> Result<Vec<u32>, Internal> {
        let mut blocks = vec![self.pattern_parts(pattern)?.1];
        let mut items = Vec::new();
        let mut found = Vec::new();
        while let Some(block) = blocks.pop() {
            let Some(rec) = self.ir.blocks.get(block as usize) else {
                continue;
            };
            items.extend(
                self.ir
                    .block_items
                    .get(rec.items.0 as usize..rec.items.1 as usize)
                    .unwrap_or_default(),
            );
            while let Some(item) = items.pop() {
                CountedWalk::CorePatternGraph.step();
                match *self.ir.item(item)? {
                    ItemRec::Placement(placement) => {
                        if let Some(used) = self.used_pattern(placement.kind) {
                            found.push(used);
                        }
                        blocks.push(placement.body);
                    }
                    ItemRec::For(rec) => blocks.push(rec.body),
                    ItemRec::Line(members) => items.extend(
                        self.ir
                            .members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default(),
                    ),
                    ItemRec::Wire(_)
                    | ItemRec::Bundle(_)
                    | ItemRec::When(_)
                    | ItemRec::Paren(_) => {}
                }
            }
        }
        found.sort_unstable();
        found.dedup();
        Ok(found)
    }

    pub fn pattern_cycles(&mut self) -> Result<(), Internal> {
        let mut edges = Vec::new();
        for pattern in 0..u32::try_from(self.ir.patterns.len()).unwrap_or(0) {
            edges.push(self.patterns_in_body(pattern)?);
        }
        let mut components = super::cycles::strongly_connected(&edges);
        for component in &mut components {
            component.sort_unstable();
        }
        components.sort_unstable();
        for component in components {
            let looped = match component.as_slice() {
                [one] => edges
                    .get(*one as usize)
                    .is_some_and(|targets| targets.contains(one)),
                _ => true,
            };
            if !looped {
                continue;
            }
            let mut names = Vec::new();
            let mut spans = Vec::new();
            for member in &component {
                if let Some(slot) = self.cyclic_patterns.get_mut(*member as usize) {
                    *slot = true;
                }
                let rec = *self
                    .ir
                    .patterns
                    .get(*member as usize)
                    .ok_or(Internal::missed("IR pattern"))?;
                let file = self.pattern_file(*member);
                names.push(format!(
                    "`{}`",
                    self.ir.name_text(rec.name).unwrap_or_default()
                ));
                spans.push(self.name_span(file, rec.name)?);
            }
            let message = if names.len() == 1 {
                format!("the pattern {} uses itself", names.join(""))
            } else {
                format!("the patterns {} use each other", names.join(", "))
            };
            let mut spans = spans.into_iter();
            let Some(first) = spans.next() else {
                continue;
            };
            let others = spans
                .map(|span| (span, "a pattern on the cycle".to_string()))
                .collect();
            self.draw_related(Code::RecursivePattern, first, others, message);
        }
        Ok(())
    }

    pub fn cyclic_pattern(&self, pattern: u32) -> bool {
        self.cyclic_patterns
            .get(pattern as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn stand_in(&self, pattern: u32) -> Stage<Option<u32>> {
        let mut at = pattern;
        loop {
            if self.cyclic_pattern(at) {
                return Ok(Ok(None));
            }
            let body = self.pattern_parts(at)?.1;
            let block = *self
                .ir
                .blocks
                .get(body as usize)
                .ok_or(Internal::missed("IR block"))?;
            let items = self
                .ir
                .block_items
                .get(block.items.0 as usize..block.items.1 as usize)
                .unwrap_or_default();
            let [item] = items else {
                return Ok(Ok(None));
            };
            let ItemRec::Placement(placement) = *self.ir.item(*item)? else {
                return Ok(Ok(None));
            };
            if !self.ir.named(placement.kind) {
                return Ok(Ok(None));
            }
            if let Some(next) = self.used_pattern(placement.kind) {
                at = next;
                continue;
            }
            let entry = *self
                .ir
                .name(placement.kind)
                .ok_or(Internal::missed("IR name"))?;
            if self.decl.get(entry.sym.0 as usize) == Some(&DeclRef::Duplicate) {
                return Ok(Ok(Some(*item)));
            }
            let text = self.ir.symbols.text(entry.sym);
            let node = self.vocabulary.node_kind_named(text).is_some()
                && self
                    .decl
                    .get(entry.sym.0 as usize)
                    .is_none_or(|decl| *decl == DeclRef::None);
            return Ok(Ok(node.then_some(*item)));
        }
    }

    pub fn duplicated_kind(&self, item: u32) -> Result<bool, Internal> {
        let kind = self.placement(item)?.kind;
        Ok(self.ir.named(kind)
            && self.ir.name(kind).is_some_and(|entry| {
                self.decl.get(entry.sym.0 as usize) == Some(&DeclRef::Duplicate)
            }))
    }

    pub fn unknown_use(&mut self, parent: u32) {
        self.unknown_kind = true;
        self.unknown_wires = true;
        if parent == NONE {
            self.unknown_top = true;
        } else {
            self.unknown_children.insert(parent);
        }
    }

    pub fn top_use(&mut self, item: u32, pattern: u32, file: u32) -> Stage<u32> {
        let placement = self.placement(item)?;
        if self.cyclic_pattern(pattern) {
            self.unknown_use(NONE);
            return Ok(Ok(NONE));
        }
        match self.stand_in(pattern)? {
            Ok(Some(_)) => {}
            Ok(None) => {
                let span = self.name_span(file, placement.kind)?;
                self.draw(
                    Code::TopLevelPattern,
                    span,
                    "only a pattern that stands in for a single node placement is used at top level",
                );
                self.unknown_use(NONE);
                return Ok(Ok(NONE));
            }
            Err(declined) => return Ok(Err(declined)),
        }
        self.stand_in_row(item, pattern, (file, NONE, NONE), true)
    }

    pub fn stand_in_row(
        &mut self,
        item: u32,
        pattern: u32,
        (file, parent, env): (u32, u32, u32),
        reachable: bool,
    ) -> Stage<u32> {
        let placement = self.placement(item)?;
        let outer = env;
        let (mut at, mut using, mut from, mut env) = (item, pattern, file, env);
        loop {
            let made = match self.use_env(at, using, (from, env), at == item)? {
                Ok(Some(made)) => made,
                Ok(None) => return Ok(Ok(NONE)),
                Err(declined) => return Ok(Err(declined)),
            };
            let body = self.pattern_parts(using)?.1;
            let block = *self
                .ir
                .blocks
                .get(body as usize)
                .ok_or(Internal::missed("IR block"))?;
            let inner = *self
                .ir
                .block_items
                .get(block.items.0 as usize)
                .ok_or(Internal::missed("IR block item"))?;
            let inner_file = self.pattern_file(using);
            if let Some(next) = self.used_pattern(self.placement(inner)?.kind) {
                (at, using, from, env) = (inner, next, inner_file, made);
                continue;
            }
            let name = if self.ir.named(placement.as_name) {
                placement.as_name
            } else {
                NONE
            };
            if self.duplicated_kind(inner)? {
                if parent == NONE {
                    self.unknown_use(NONE);
                } else if reachable && let Some(entry) = self.ir.name(name).copied() {
                    let span = self.span(file, entry.lo, entry.hi);
                    let key = u32::try_from(self.instances.len()).unwrap_or(NONE);
                    self.placeholder_uses.push((parent, name, span, key));
                }
            }
            let made_row = self.named_row(
                inner,
                (inner_file, parent, made),
                (name, file, reachable && name != NONE),
                item,
            )?;
            if let Ok(row) = made_row
                && row != NONE
                && outer != NONE
                && self.pattern_items.contains_key(&item)
            {
                self.env_rows.insert((item, outer), row);
            }
            return Ok(made_row);
        }
    }
}

impl Core<'_> {
    pub fn unrun_use(&mut self, placement: PlacementRec, pattern: u32, file: u32) -> Stage<()> {
        if placement.body != NONE || placement.inline {
            return Ok(Err(Declined(Reason::PatternUse)));
        }
        let span = self.span(file, placement.lo, placement.hi);
        match self.use_sources(placement, pattern, (file, NONE), span)? {
            Ok(_) => Ok(Ok(())),
            Err(declined) => Ok(Err(declined)),
        }
    }
}

impl Core<'_> {
    pub fn env_file(&self, env: u32, file: u32) -> u32 {
        let mut at = env;
        while let Some(record) = self.envs.get(at as usize) {
            if record.use_item != NONE {
                return self
                    .bodies
                    .get(record.inst as usize)
                    .and_then(|body| self.pattern_at(body.function))
                    .map_or(file, |pattern| self.pattern_file(pattern));
            }
            at = record.parent;
        }
        file
    }
}

impl Core<'_> {
    pub fn pattern_names(&mut self) -> Result<(), Internal> {
        let mut read: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
        let start = self.patterns_start();
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) != Some(&Op::Name) {
                continue;
            }
            let owner = self.body_of.get(local).copied().unwrap_or(NONE);
            let Some(pattern) = owner
                .checked_sub(start)
                .filter(|at| (*at as usize) < self.ir.patterns.len())
            else {
                continue;
            };
            if matches!(
                self.bind.get(local),
                Some(DeclRef::Param(_) | DeclRef::Instance(_))
            ) {
                continue;
            }
            let name = *self.ir.a.get(local).ok_or(Internal::missed("IR name"))?;
            if let Some(entry) = self.ir.name(name) {
                read.entry(pattern).or_default().push(entry.sym.0);
            }
        }
        for pattern in 0..u32::try_from(self.ir.patterns.len()).unwrap_or(0) {
            CountedWalk::CorePatternGraph.step();
            let declined = self.pattern_items_declined(pattern)?;
            let reads = read.remove(&pattern).unwrap_or_default();
            let names = self.pattern_clashes(pattern)?;
            let named = declined || names.iter().any(|sym| reads.contains(sym));
            if let Some(slot) = self.named_patterns.get_mut(pattern as usize) {
                *slot = named;
            }
        }
        for local in 0..self.ir.op.len() {
            if self.ir.op.get(local) == Some(&Op::Name)
                && let Some(DeclRef::Instance(item)) = self.bind.get(local)
                && let Some(pattern) = self.pattern_items.get(item).copied()
            {
                self.own_reads.insert(pattern);
            }
        }
        Ok(())
    }

    fn pattern_clashes(&mut self, pattern: u32) -> Result<Vec<u32>, Internal> {
        let (params, body) = self.pattern_parts(pattern)?;
        let file = self.pattern_file(pattern);
        let own = self.stand_in_item(body)?;
        let mut visible: Vec<(u32, symphonyscript_core::Span)> = Vec::new();
        for binder in params.0..params.1 {
            let rec = *self.ir.binder(binder)?;
            if let Some(entry) = self.ir.name(rec.name).filter(|_| self.ir.named(rec.name)) {
                visible.push((entry.sym.0, self.span(file, entry.lo, entry.hi)));
            }
        }
        let params = visible.len();
        let mut all = Vec::new();
        let mut blocks = vec![(body, visible)];
        while let Some((block, outer)) = blocks.pop() {
            let Some(found) = self.ir.blocks.get(block as usize).copied() else {
                continue;
            };
            let mut items: Vec<u32> = self
                .ir
                .block_items
                .get(found.items.0 as usize..found.items.1 as usize)
                .unwrap_or_default()
                .to_vec();
            let mut at = 0;
            while let Some(item) = items.get(at).copied() {
                at += 1;
                if let ItemRec::Line(members) = *self.ir.item(item)? {
                    items.extend(
                        self.ir
                            .members
                            .get(members.0 as usize..members.1 as usize)
                            .unwrap_or_default(),
                    );
                }
            }
            let mut here: Vec<(u32, symphonyscript_core::Span)> = Vec::new();
            let mut bodies = Vec::new();
            for item in items {
                let ItemRec::Placement(placement) = *self.ir.item(item)? else {
                    continue;
                };
                bodies.push(placement.body);
                if !self.ir.named(placement.as_name) {
                    continue;
                }
                let entry = *self
                    .ir
                    .name(placement.as_name)
                    .ok_or(Internal::missed("IR name"))?;
                let span = self.span(file, entry.lo, entry.hi);
                let text = self.ir.symbols.text(entry.sym).to_string();
                if item != own {
                    all.push(entry.sym.0);
                }
                self.pattern_items.insert(item, pattern);
                let param = outer
                    .iter()
                    .take(params)
                    .find(|(sym, _)| *sym == entry.sym.0)
                    .map(|(_, span)| span.clone());
                let other = here
                    .iter()
                    .chain(outer.iter().skip(params).rev())
                    .find(|(sym, _)| *sym == entry.sym.0)
                    .map(|(_, span)| span.clone());
                if let Some(repeated) = param {
                    self.draw_related(
                        Code::ChildNameClash,
                        span.clone(),
                        vec![(repeated, "the name it repeats".to_string())],
                        format!("`{text}` names a pattern parameter, a pattern `let` or a `for` variable visible here"),
                    );
                } else if let Some(earlier) = other {
                    self.draw_related(
                        Code::ChildNameClash,
                        span.clone(),
                        vec![(earlier, "the other name".to_string())],
                        format!("`{text}` names another child or wire of this body or of an enclosing one"),
                    );
                }
                here.push((entry.sym.0, span));
            }
            for inner in bodies {
                if inner == NONE || inner == ABSENT {
                    continue;
                }
                let mut seen = outer.clone();
                seen.extend(here.iter().cloned());
                blocks.push((inner, seen));
            }
        }
        Ok(all)
    }
}

impl Core<'_> {
    pub fn argument_of(
        &self,
        node: u32,
        position: u32,
        env: u32,
    ) -> Result<Option<(u32, u32)>, Internal> {
        let owner = self.binding_owner(node)?;
        let mut at = env;
        while let Some(record) = self.envs.get(at as usize) {
            let owned = self
                .bodies
                .get(record.inst as usize)
                .is_some_and(|body| body.function == owner);
            if record.use_item != NONE && owned {
                let Some((root, caller)) = self
                    .use_args
                    .get(&at)
                    .and_then(|args| args.get(position as usize))
                    .copied()
                else {
                    return Ok(None);
                };
                if root == NONE {
                    return Ok(None);
                }
                if self.default_nodes.contains_key(&root) {
                    return Ok(Some((root, at)));
                }
                return Ok(Some((root, caller)));
            }
            at = record.parent;
        }
        Ok(None)
    }
}
