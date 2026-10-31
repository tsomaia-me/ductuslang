use std::collections::BTreeMap;
use std::rc::Rc;

use symphonyscript_core::{BinaryOp, Builtin};

use super::answer::Answer;
use super::owner_reads::OwnerReads;
use super::owners::Owners;
use super::reads::Reads;
use super::scoped_reads::ScopedReads;
use super::variable::Variable;
use crate::semantics::Internal;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::resolve::Found;
use crate::semantics::collection::{BuiltinName, Decl, Namespace, strongly_connected};
use crate::semantics::ir::{
    Args, AttrValue, BodyItem, ExprId, ExprKind, FnId, LetId, PlacementId, StateId, TypeRef,
};

enum Position<'p> {
    Declared(Declared<'p>),
    Scope(ScopeId),
}

struct Declared<'p> {
    namespace: &'p Namespace,
    owners: &'p Owners,
    locals: BTreeMap<&'p str, Option<ExprId>>,
    every_arm: bool,
    known: &'p [Option<Answer>],
    consulted: Vec<usize>,
    settled: bool,
}

enum Step {
    Missing(Vec<ExprId>),
    Known(Answer),
}

enum PathStart {
    Instance(PlacementId),
    Chosen,
}

type Done = FastMap<ExprId, Answer>;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn let_reads_cell(
        &mut self,
        id: LetId,
    ) -> Result<Reads, Internal> {
        if let Some(bound) = self.memo.lets.get(&(self.source_global, LetRef::Top(id))) {
            return Ok(bound_reads(bound));
        }
        Ok(self.owner_answer(id.index())?.reads)
    }

    pub(in crate::semantics::analysis) fn function_reads_cell(
        &mut self,
        id: FnId,
    ) -> Result<Reads, Internal> {
        Ok(self.owner_answer(self.hir.lets.len() + id.index())?.reads)
    }

    pub(in crate::semantics::analysis) fn expr_reads_cell(
        &mut self,
        root: ExprId,
        scope: ScopeId,
        known: &mut ScopedReads,
    ) -> Result<Reads, Internal> {
        let done = known.scopes.entry(scope).or_default();
        Ok(self.answer(root, &mut Position::Scope(scope), done)?.reads)
    }

    fn answer(
        &mut self,
        root: ExprId,
        position: &mut Position<'_>,
        done: &mut Done,
    ) -> Result<Answer, Internal> {
        let hir = self.hir;
        let mut stack = vec![root];
        while let Some(expr) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::AnswersReadsCell.step();
            if done.contains_key(&expr) {
                continue;
            }
            match self.read_step(expr, &hir.expr(expr)?.kind, done, position)? {
                Step::Known(answer) => {
                    done.insert(expr, answer);
                }
                Step::Missing(children) => {
                    if children.is_empty() {
                        return Err(Internal::missed("read operand"));
                    }
                    stack.push(expr);
                    stack.extend(children);
                }
            }
        }
        done.get(&root).copied().ok_or(Internal::missed("read"))
    }

    fn read_step(
        &mut self,
        expr: ExprId,
        kind: &ExprKind,
        done: &Done,
        position: &mut Position<'_>,
    ) -> Result<Step, Internal> {
        Ok(match kind {
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Variant { .. }
            | ExprKind::To => Step::Known(Answer::VALUE),
            ExprKind::Name(name) => self.name_step(name, done, position)?,
            ExprKind::Str(_) | ExprKind::Convert(..) | ExprKind::Array(_) | ExprKind::Unary(..) => {
                fixed(joined(done, &kind.children()))
            }
            ExprKind::Index(..) | ExprKind::IndexFromEnd(..) => {
                with_variable(joined(done, &kind.children()), Variable::Unknown)
            }
            ExprKind::Slice { base, lower, upper } => match joined(done, &kind.children()) {
                Step::Known(answer) => {
                    let base = done.get(base).copied().unwrap_or(Answer::VALUE);
                    let bounds_read = lower.iter().chain(upper.iter()).any(|bound| {
                        done.get(&bound.value)
                            .is_some_and(|bound| bound.reads.is_cell())
                    });
                    let variable = match base.variable {
                        Variable::Fixed if bounds_read => Variable::Whole,
                        variable => variable,
                    };
                    Step::Known(Answer::of(answer.reads, variable))
                }
                missing @ Step::Missing(_) => missing,
            },
            ExprKind::Binary(op, ..) => match joined(done, &kind.children()) {
                Step::Known(answer) if *op == BinaryOp::Add => Step::Known(answer),
                Step::Known(answer) => Step::Known(Answer::of(answer.reads, Variable::Fixed)),
                missing @ Step::Missing(_) => missing,
            },
            ExprKind::History { target, .. } => match joined(done, &kind.children()) {
                Step::Known(answer) => {
                    let cell = self.name_step(&target.text, done, position)?;
                    let target = match cell {
                        Step::Known(target) => target.reads,
                        Step::Missing(_) => Reads::Cell,
                    };
                    Step::Known(Answer::of(answer.reads.join(target), Variable::Unknown))
                }
                missing @ Step::Missing(_) => missing,
            },
            ExprKind::Stepped(target) => {
                let reads = match self.name_step(&target.text, done, position)? {
                    Step::Known(target) => target.reads,
                    Step::Missing(_) => Reads::Cell,
                };
                Step::Known(Answer::of(reads, Variable::Fixed))
            }
            ExprKind::Field(base, _) => match self.path_start(expr, position)? {
                Some(PathStart::Instance(placement)) => Step::Known(Answer::of(
                    self.attribute_reads(expr, placement, position)?,
                    Variable::Unknown,
                )),
                Some(PathStart::Chosen) | None => match joined(done, &[*base]) {
                    Step::Known(answer) => Step::Known(Answer::of(
                        answer.reads.join(Reads::Chosen),
                        Variable::Unknown,
                    )),
                    missing @ Step::Missing(_) => missing,
                },
            },
            ExprKind::Call(callee, args) => self.call_step(*callee, args, done, position)?,
            ExprKind::If(condition, then, otherwise) => {
                let taken = match self.hir.expr(*condition)?.kind {
                    ExprKind::Bool(true) => Some(*then),
                    ExprKind::Bool(false) => Some(*otherwise),
                    ExprKind::Error
                    | ExprKind::Number(..)
                    | ExprKind::Str(_)
                    | ExprKind::Name(_)
                    | ExprKind::Variant { .. }
                    | ExprKind::To
                    | ExprKind::Convert(..)
                    | ExprKind::Array(_)
                    | ExprKind::Field(..)
                    | ExprKind::Index(..)
                    | ExprKind::IndexFromEnd(..)
                    | ExprKind::Slice { .. }
                    | ExprKind::History { .. }
                    | ExprKind::Stepped(_)
                    | ExprKind::Call(..)
                    | ExprKind::Unary(..)
                    | ExprKind::Binary(..)
                    | ExprKind::If(..)
                    | ExprKind::Match { .. }
                    | ExprKind::For(_) => None,
                };
                self.choice_step(*condition, &[*then, *otherwise], taken, done, position)?
            }
            ExprKind::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let values: Vec<ExprId> =
                    arms.iter().map(|arm| arm.value).chain(*otherwise).collect();
                self.choice_step(*scrutinee, &values, None, done, position)?
            }
            ExprKind::For(for_expr) => match joined(done, &for_expr.children()) {
                Step::Known(answer) if answer.reads.is_cell() => {
                    Step::Known(Answer::of(answer.reads, Variable::Whole))
                }
                Step::Known(answer) => Step::Known(Answer::of(answer.reads, Variable::Unknown)),
                missing @ Step::Missing(_) => missing,
            },
        })
    }

    fn choice_step(
        &self,
        selector: ExprId,
        arms: &[ExprId],
        taken: Option<ExprId>,
        done: &Done,
        position: &Position<'_>,
    ) -> Result<Step, Internal> {
        if let Some(taken) = taken {
            let answer = match joined(done, &[taken]) {
                Step::Known(answer) => answer,
                missing @ Step::Missing(_) => return Ok(missing),
            };
            return Ok(Step::Known(match position {
                Position::Declared(Declared {
                    every_arm: true, ..
                }) => {
                    let mut parts = vec![selector];
                    parts.extend(arms);
                    match joined(done, &parts) {
                        Step::Known(all) => Answer::of(all.reads, answer.variable),
                        missing @ Step::Missing(_) => return Ok(missing),
                    }
                }
                Position::Declared(_) | Position::Scope(_) => answer,
            }));
        }
        let mut parts = vec![selector];
        parts.extend(arms);
        if let missing @ Step::Missing(_) = joined(done, &parts) {
            return Ok(missing);
        }
        let answers: Vec<Answer> = arms
            .iter()
            .filter_map(|arm| done.get(arm).copied())
            .collect();
        let selected = done.get(&selector).copied().unwrap_or(Answer::VALUE).reads;
        let every_arm = match position {
            Position::Declared(declared) => declared.every_arm,
            Position::Scope(_) => false,
        };
        let all_reads = Reads::of_all(answers.iter().map(|answer| answer.reads));
        let arms_agree = answers.iter().all(|answer| answer.reads == all_reads);
        let reads = if every_arm || selected.is_cell() || arms_agree {
            selected.join(all_reads)
        } else {
            selected.join(Reads::Chosen)
        };
        let variables = answers.iter().map(|answer| answer.variable);
        let variable = if selected.is_cell() {
            let mut variable = variables.fold(Variable::Fixed, Variable::either);
            if variable == Variable::Fixed && self.literal_lengths_differ(arms)? {
                variable = Variable::Whole;
            }
            variable
        } else {
            Variable::agreed(variables)
        };
        Ok(Step::Known(Answer::of(reads, variable)))
    }

    fn literal_lengths_differ(&self, arms: &[ExprId]) -> Result<bool, Internal> {
        let mut lengths = Vec::with_capacity(arms.len());
        for arm in arms {
            if let ExprKind::Array(elements) = &self.hir.expr(*arm)?.kind {
                lengths.push(elements.len());
            }
        }
        Ok(lengths.iter().any(|length| Some(length) != lengths.first()))
    }

    fn call_step(
        &mut self,
        callee: ExprId,
        args: &Args,
        done: &Done,
        position: &mut Position<'_>,
    ) -> Result<Step, Internal> {
        let values = args.values();
        if !self.measures_length(callee, position)? {
            let mut parts = vec![callee];
            parts.extend(values);
            return Ok(with_variable(joined(done, &parts), Variable::Unknown));
        }
        let answer = match joined(done, &values) {
            Step::Known(answer) => answer,
            missing @ Step::Missing(_) => return Ok(missing),
        };
        let measured = values
            .first()
            .and_then(|value| done.get(value))
            .map_or(Variable::Fixed, |measured| measured.variable);
        let reads = if answer.reads.is_value() {
            Reads::Value
        } else {
            match measured {
                Variable::Whole => answer.reads,
                Variable::Fixed => Reads::Value,
                Variable::Unknown => Reads::Chosen,
            }
        };
        Ok(Step::Known(Answer::of(reads, Variable::Fixed)))
    }

    fn measures_length(
        &mut self,
        callee: ExprId,
        position: &Position<'_>,
    ) -> Result<bool, Internal> {
        let ExprKind::Name(name) = &self.hir.expr(callee)?.kind else {
            return Ok(false);
        };
        let decl = match position {
            Position::Declared(declared) => {
                if declared.locals.contains_key(name.as_str()) {
                    return Ok(false);
                }
                declared.namespace.get(name)
            }
            Position::Scope(scope) => found_decl(self.find(name, *scope)?),
        };
        Ok(matches!(
            decl,
            Some(Decl::Builtin(BuiltinName::Function(Builtin::Len)))
        ))
    }

    fn name_step(
        &mut self,
        name: &str,
        done: &Done,
        position: &mut Position<'_>,
    ) -> Result<Step, Internal> {
        let declared = match position {
            Position::Declared(declared) => declared,
            Position::Scope(scope) => {
                let scope = *scope;
                return Ok(Step::Known(self.scoped_name(name, scope)?));
            }
        };
        if let Some(local) = declared.locals.get(name) {
            return Ok(match local {
                None => Step::Known(Answer::VALUE),
                Some(value) => match done.get(value) {
                    Some(answer) => Step::Known(*answer),
                    None => Step::Missing(vec![*value]),
                },
            });
        }
        let owner = match declared.namespace.get(name) {
            Some(decl @ (Decl::Input(_) | Decl::State(_))) => {
                if let Decl::State(id) = decl
                    && self.hir.state(id)?.ty.is_none()
                {
                    declared.settled = true;
                }
                return Ok(Step::Known(self.cell_answer(decl)?));
            }
            Some(Decl::Let(id)) => {
                if self.hir.is_prelude_let(id) {
                    return Ok(Step::Known(Answer::VALUE));
                }
                let answer = declared.consult(id.index());
                return Ok(Step::Known(self.resolved_let(id, answer)));
            }
            Some(Decl::Fn(id)) => {
                if self.hir.is_prelude_fn(id) {
                    return Ok(Step::Known(Answer::VALUE));
                }
                declared.owners.function(id)
            }
            Some(
                Decl::Native(_)
                | Decl::Pattern(_)
                | Decl::Instance(_)
                | Decl::Builtin(_)
                | Decl::Node(_)
                | Decl::Wire(_)
                | Decl::Enum(_),
            )
            | None => return Ok(Step::Known(Answer::VALUE)),
        };
        let answer = declared.consult(owner);
        Ok(Step::Known(Answer::of(answer.reads, Variable::Fixed)))
    }

    fn resolved_let(&self, id: LetId, answer: Answer) -> Answer {
        match self.memo.lets.get(&(self.source_global, LetRef::Top(id))) {
            Some(bound) => Answer::of(bound_reads(bound), answer.variable),
            None => answer,
        }
    }

    fn cell_answer(&self, decl: Decl) -> Result<Answer, Internal> {
        let variable = match decl {
            Decl::Input(id) => match self.hir.input(id)?.ty.as_ref() {
                Some(ty) if ty.capacity.is_some() => Variable::Whole,
                Some(_) | None => Variable::Fixed,
            },
            Decl::State(id) => match self.hir.state(id)?.ty.as_ref() {
                Some(ty) if ty.capacity.is_some() => Variable::Whole,
                Some(_) => Variable::Fixed,
                None => self.settled_variable(id),
            },
            Decl::Let(_)
            | Decl::Fn(_)
            | Decl::Native(_)
            | Decl::Pattern(_)
            | Decl::Instance(_)
            | Decl::Builtin(_)
            | Decl::Node(_)
            | Decl::Wire(_)
            | Decl::Enum(_) => return Ok(Answer::VALUE),
        };
        Ok(Answer::of(Reads::Cell, variable))
    }

    fn settled_variable(&self, state: StateId) -> Variable {
        let Some(BoundVal::Scalar(Bound::Cell(binding))) = self.memo.states.get(&state) else {
            return Variable::Unknown;
        };
        if !self.memo.settled.contains(binding) {
            return Variable::Unknown;
        }
        match self.memo.binding_layouts.get(binding) {
            Some(layout) if layout.variable => Variable::Whole,
            Some(_) | None => Variable::Fixed,
        }
    }

    fn scoped_name(&mut self, name: &str, scope: ScopeId) -> Result<Answer, Internal> {
        Ok(match self.find(name, scope)? {
            Found::Decl(decl @ (Decl::Input(_) | Decl::State(_))) => self.cell_answer(decl)?,
            Found::Query(Query::Input(_) | Query::State(_)) => {
                Answer::of(Reads::Cell, Variable::Unknown)
            }
            Found::Decl(Decl::Let(id)) => {
                if self.hir.is_prelude_let(id) {
                    Answer::VALUE
                } else {
                    let answer = self.owner_answer(id.index())?;
                    self.resolved_let(id, answer)
                }
            }
            Found::Decl(Decl::Fn(id)) => {
                if self.hir.is_prelude_fn(id) {
                    Answer::VALUE
                } else {
                    Answer::of(self.function_reads_cell(id)?, Variable::Fixed)
                }
            }
            Found::Query(query) => match self.bound(query, &mut Vec::new()) {
                Some(bound) => Answer::of(bound_reads(&bound), Variable::Unknown),
                None => match query {
                    Query::Let(_, LetRef::Top(id)) => {
                        Answer::of(self.let_reads_cell(id)?, Variable::Unknown)
                    }
                    Query::Expr(..)
                    | Query::Let(_, LetRef::Local(_))
                    | Query::Param(..)
                    | Query::Input(_)
                    | Query::State(_)
                    | Query::Attr(..)
                    | Query::Destination(_)
                    | Query::Sig(_)
                    | Query::Eval(..) => Answer::of(Reads::Value, Variable::Unknown),
                },
            },
            Found::Val(val) => {
                let reactive = val
                    .nodes()
                    .iter()
                    .any(|node| self.memo.sigs.get(node).is_some_and(|sig| sig.reactive));
                Answer::of(
                    if reactive { Reads::Cell } else { Reads::Value },
                    Variable::Unknown,
                )
            }
            Found::Decl(
                Decl::Native(_)
                | Decl::Pattern(_)
                | Decl::Instance(_)
                | Decl::Builtin(_)
                | Decl::Node(_)
                | Decl::Wire(_)
                | Decl::Enum(_),
            )
            | Found::Item(_)
            | Found::Placeholder
            | Found::Nothing => Answer::VALUE,
        })
    }

    fn path_start(
        &mut self,
        field: ExprId,
        position: &Position<'_>,
    ) -> Result<Option<PathStart>, Internal> {
        let mut current = field;
        let start = loop {
            crate::semantics::counted_walk::CountedWalk::AnswersPathStart.step();
            match &self.hir.expr(current)?.kind {
                ExprKind::Field(base, _) => current = *base,
                ExprKind::Name(start) => break start,
                ExprKind::To => return Ok(Some(PathStart::Chosen)),
                ExprKind::Error
                | ExprKind::Number(..)
                | ExprKind::Bool(_)
                | ExprKind::Str(_)
                | ExprKind::Variant { .. }
                | ExprKind::Convert(..)
                | ExprKind::Array(_)
                | ExprKind::Index(..)
                | ExprKind::IndexFromEnd(..)
                | ExprKind::Slice { .. }
                | ExprKind::History { .. }
                | ExprKind::Stepped(_)
                | ExprKind::Call(..)
                | ExprKind::Unary(..)
                | ExprKind::Binary(..)
                | ExprKind::If(..)
                | ExprKind::Match { .. }
                | ExprKind::For(_) => return Ok(None),
            }
        };
        let decl = match position {
            Position::Declared(declared) => {
                if declared.locals.contains_key(start.as_str()) {
                    return Ok(Some(PathStart::Chosen));
                }
                declared.namespace.get(start)
            }
            Position::Scope(scope) => found_decl(self.find(start, *scope)?),
        };
        Ok(Some(match decl {
            Some(Decl::Instance(placement)) => PathStart::Instance(placement),
            Some(
                Decl::Input(_)
                | Decl::Let(_)
                | Decl::State(_)
                | Decl::Fn(_)
                | Decl::Native(_)
                | Decl::Pattern(_)
                | Decl::Builtin(_)
                | Decl::Node(_)
                | Decl::Wire(_)
                | Decl::Enum(_),
            )
            | None => PathStart::Chosen,
        }))
    }

    fn attribute_reads(
        &mut self,
        field: ExprId,
        start: PlacementId,
        position: &mut Position<'_>,
    ) -> Result<Reads, Internal> {
        let hir = self.hir;
        let mut segments = Vec::new();
        let mut current = field;
        while let ExprKind::Field(base, segment) = &hir.expr(current)?.kind {
            segments.push(segment.text.as_str());
            current = *base;
        }
        let Some((attribute, steps)) = segments.split_first() else {
            return Ok(Reads::Chosen);
        };
        let mut placement = start;
        for step in steps.iter().rev() {
            let Some(body) = hir.placement(placement)?.body else {
                return Ok(Reads::Chosen);
            };
            let mut child = None;
            for item in &hir.block(body)?.items {
                if let BodyItem::Placement(id) = item
                    && hir
                        .placement(*id)?
                        .name
                        .as_ref()
                        .is_some_and(|name| name.text == *step)
                {
                    child = Some(*id);
                }
            }
            let Some(child) = child else {
                return Ok(Reads::Chosen);
            };
            placement = child;
        }
        let record = hir.placement(placement)?;
        let pattern_use = record.kind.as_ref().is_some_and(|kind| {
            matches!(
                self.collected.source.get(&kind.text),
                Some(Decl::Pattern(_))
            )
        });
        if pattern_use {
            return Ok(Reads::Chosen);
        }
        let Some(index) = record
            .attrs
            .iter()
            .position(|attr| attr.name.text == *attribute)
        else {
            return Ok(if record.default.is_some() {
                Reads::Chosen
            } else {
                Reads::Value
            });
        };
        let value = record
            .attrs
            .get(index)
            .map(|attr| attr.value)
            .ok_or(Internal::missed("attribute"))?;
        if let AttrValue::Set(_) = value {
            return Ok(Reads::Value);
        }
        Ok(match position {
            Position::Declared(declared) => {
                let owner = declared.owners.attribute(placement, index)?;
                declared.consult(owner).reads
            }
            Position::Scope(_) => {
                let known = self.owner_reads_known()?;
                let owner = known.owners.attribute(placement, index)?;
                known
                    .answers
                    .get(owner)
                    .copied()
                    .ok_or(Internal::missed("read owner"))?
                    .reads
            }
        })
    }

    fn owner_answer(&mut self, owner: usize) -> Result<Answer, Internal> {
        self.owner_reads_known()?
            .answers
            .get(owner)
            .copied()
            .ok_or(Internal::missed("read owner"))
    }

    fn owner_reads_known(&mut self) -> Result<Rc<OwnerReads>, Internal> {
        let stamp = (self.memo.settle_epoch, self.memo.settled.len());
        if let Some(known) = &self.memo.owner_reads
            && known.stamp.is_none_or(|settled| settled == stamp)
        {
            return Ok(Rc::clone(known));
        }
        let owners = Owners::of(self.hir);
        let (answers, settled) = self.owner_answers(&owners)?;
        let known = Rc::new(OwnerReads {
            owners,
            answers,
            stamp: settled.then_some(stamp),
        });
        self.memo.owner_reads = Some(Rc::clone(&known));
        Ok(known)
    }

    fn owner_answers(&mut self, owners: &Owners) -> Result<(Vec<Answer>, bool), Internal> {
        let count = owners.count();
        let unknown = vec![None; count];
        let mut edges = Vec::with_capacity(count);
        let mut settled = false;
        for owner in 0..count {
            let (_, consulted, reads_settled) = self.owner_step(owner, owners, &unknown)?;
            edges.push(consulted);
            settled |= reads_settled;
        }
        let mut known: Vec<Option<Answer>> = vec![None; count];
        for component in strongly_connected(&edges)? {
            let mut answer: Option<Answer> = None;
            for member in &component {
                let found = self.owner_step(*member, owners, &known)?.0;
                answer = Some(answer.map_or(found, |answer| answer.join(found)));
            }
            for member in component {
                if let Some(slot) = known.get_mut(member) {
                    *slot = answer;
                }
            }
        }
        let answers = known
            .into_iter()
            .map(|answer| answer.unwrap_or(Answer::VALUE))
            .collect();
        Ok((answers, settled))
    }

    fn owner_step(
        &mut self,
        owner: usize,
        owners: &Owners,
        known: &[Option<Answer>],
    ) -> Result<(Answer, Vec<usize>, bool), Internal> {
        let hir = self.hir;
        let Some((roots, locals, every_arm)) = owners.roots(hir, owner)? else {
            return Ok((Answer::VALUE, Vec::new(), false));
        };
        let annotated = owner < hir.lets.len()
            && hir
                .lets
                .get(owner)
                .is_some_and(|decl| annotated_capacity(decl.ty.as_ref()));
        let mut position = Position::Declared(Declared {
            namespace: &self.collected.source,
            owners,
            locals,
            every_arm,
            known,
            consulted: Vec::new(),
            settled: false,
        });
        let mut done = Done::default();
        let mut answer: Option<Answer> = None;
        for root in roots {
            let found = self.answer(root, &mut position, &mut done)?;
            answer = Some(answer.map_or(found, |answer| answer.join(found)));
        }
        let mut answer = answer.unwrap_or(Answer::VALUE);
        if annotated {
            answer.variable = Variable::Whole;
        }
        Ok(match position {
            Position::Declared(declared) => (answer, declared.consulted, declared.settled),
            Position::Scope(_) => (answer, Vec::new(), false),
        })
    }
}

impl Declared<'_> {
    fn consult(&mut self, owner: usize) -> Answer {
        self.consulted.push(owner);
        self.known
            .get(owner)
            .copied()
            .flatten()
            .unwrap_or(Answer::VALUE)
    }
}

fn found_decl(found: Found) -> Option<Decl> {
    match found {
        Found::Decl(decl) => Some(decl),
        Found::Val(_) | Found::Query(_) | Found::Item(_) | Found::Placeholder | Found::Nothing => {
            None
        }
    }
}

fn joined(done: &Done, parts: &[ExprId]) -> Step {
    let missing: Vec<ExprId> = parts
        .iter()
        .filter(|part| !done.contains_key(part))
        .copied()
        .collect();
    if !missing.is_empty() {
        return Step::Missing(missing);
    }
    let mut answer = Answer::VALUE;
    for part in parts {
        if let Some(found) = done.get(part) {
            answer = answer.join(*found);
        }
    }
    Step::Known(answer)
}

fn fixed(step: Step) -> Step {
    with_variable(step, Variable::Fixed)
}

fn with_variable(step: Step, variable: Variable) -> Step {
    match step {
        Step::Known(answer) => Step::Known(Answer::of(answer.reads, variable)),
        missing @ Step::Missing(_) => missing,
    }
}

fn annotated_capacity(ty: Option<&TypeRef>) -> bool {
    ty.is_some_and(|ty| ty.capacity.is_some())
}

fn bound_reads(bound: &BoundVal) -> Reads {
    let cell = match bound {
        BoundVal::Scalar(Bound::Cell(_)) => true,
        BoundVal::Array(array) => array.1.iter().any(|item| matches!(item, Bound::Cell(_))),
        BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error => false,
    };
    if cell { Reads::Cell } else { Reads::Value }
}
