use std::collections::BTreeMap;
use std::rc::Rc;

use symphonyscript_core::{BinaryOp, Builtin, UnaryOp};

use super::bind_fault::BindFault;
use super::bound_sources::bound_sources;
use super::defaulted::defaulted;
use super::found::Found;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::is_float::is_float;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::local_name::LocalName;
use crate::semantics::analysis::locals::Locals;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::unify_kinds::unify_kinds;
use crate::semantics::analysis::val::Val;
use crate::semantics::collection::{BuiltinName, Decl};
use crate::semantics::ir::{Args, ExprId, ExprKind, FnDecl, FnId, Loc};
use crate::semantics::reporting::grouped;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn call(
        &mut self,
        callee: ExprId,
        args: &Args,
        expr: ExprId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let callee_expr = self.hir.expr(callee)?;
        let ExprKind::Name(name) = &callee_expr.kind else {
            if !matches!(callee_expr.kind, ExprKind::Error) {
                self.error(
                    Code::NotCallable,
                    callee_expr.loc,
                    scope,
                    "only a function or a built-in is called",
                )?;
            }
            return Ok(Some(Val::Error));
        };
        Ok(match self.find(name, scope)? {
            Found::Decl(Decl::Builtin(BuiltinName::Function(builtin))) => {
                self.builtin_call(builtin, args, loc, scope, need)?
            }
            Found::Decl(Decl::Fn(function)) => {
                self.function_call(function, args, expr, loc, scope, need)?
            }
            Found::Decl(Decl::Native(native)) => {
                self.native_call(native, args, loc, scope, need)?
            }
            Found::Nothing => {
                if self.variants.owner(name).is_some() {
                    self.unplaced_variant_at(Some(name), callee_expr.loc, scope)?;
                }
                Some(Val::Error)
            }
            Found::Placeholder => Some(Val::Error),
            Found::Val(_) | Found::Query(_) | Found::Item(_) | Found::Decl(_) => {
                self.error(
                    Code::NotCallable,
                    callee_expr.loc,
                    scope,
                    format!("`{name}` is not a function"),
                )?;
                Some(Val::Error)
            }
        })
    }

    fn builtin_call(
        &mut self,
        builtin: Builtin,
        args: &Args,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let Args::Positional(values) = args else {
            self.error(
                Code::NamedBuiltinArguments,
                loc,
                scope,
                format!(
                    "the built-in `{}` takes positional arguments only",
                    builtin.name()
                ),
            )?;
            return Ok(Some(Val::Error));
        };
        let vals: Vec<Option<Val>> = values
            .iter()
            .map(|value| self.val(Query::Expr(*value, scope), need))
            .collect();
        if vals.iter().any(Option::is_none) {
            return Ok(None);
        }
        let arity = builtin_arity(builtin);
        if vals.len() == arity
            && matches!(builtin, Builtin::Len | Builtin::Bytes)
            && let Some(val) = vals.into_iter().flatten().next()
        {
            return match builtin {
                Builtin::Len => self.length_val(val, loc, scope, need),
                Builtin::Abs
                | Builtin::Min
                | Builtin::Max
                | Builtin::Clamp
                | Builtin::Floor
                | Builtin::Ceil
                | Builtin::Round
                | Builtin::Sqrt
                | Builtin::Sin
                | Builtin::Cos
                | Builtin::Tan
                | Builtin::Exp
                | Builtin::Log
                | Builtin::Lerp
                | Builtin::Rand
                | Builtin::Bytes => Ok(Some(self.bytes_val(val, loc, scope)?)),
            };
        }
        let vals: Vec<Option<Val>> = values
            .iter()
            .map(|value| self.val(Query::Expr(*value, scope), need))
            .collect();
        if vals.len() != arity {
            self.error(
                Code::ArgumentCount,
                loc,
                scope,
                format!(
                    "`{}` takes {arity} argument{}, not {}",
                    builtin.name(),
                    if arity == 1 { "" } else { "s" },
                    vals.len()
                ),
            )?;
            return Ok(Some(Val::Error));
        }
        let mut operands = Vec::new();
        for val in vals.into_iter().flatten() {
            self.single_operand(
                &val,
                Code::ArrayOperand,
                "built-ins apply to single values, not arrays",
            );
            match val {
                Val::Scalar(node) => operands.push(node),
                Val::Array(_) => {
                    self.error(
                        Code::ArrayOperand,
                        loc,
                        scope,
                        "built-ins apply to single values, not arrays",
                    )?;
                    return Ok(Some(Val::Error));
                }
                Val::Error => return Ok(Some(Val::Error)),
            }
        }
        Ok(Some(self.scalar(
            Op::Builtin(builtin, operands),
            loc,
            scope,
        )?))
    }

    fn function_call(
        &mut self,
        function: FnId,
        args: &Args,
        expr: ExprId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let decl = self.hir.fn_decl(function)?;
        let Some(sources) = self.bind_arguments(decl, args, loc, scope)? else {
            return Ok(Some(Val::Error));
        };
        if self.in_name_probe(scope)? {
            let mut pending = false;
            for source in &sources {
                if let ParamSource::Arg { expr, scope } = source {
                    pending |= self.val(Query::Expr(*expr, *scope), need).is_none();
                }
            }
            pending |= self.named_function(function, loc, scope, need)?.is_none();
            return Ok((!pending).then_some(Val::Error));
        }
        let recursive = self.is_recursive(function)?;
        if recursive && self.bounded_cycle(function)? {
            match self.call_reads_cell(function, &sources, need)? {
                None => return Ok(None),
                Some(true) => {
                    return self.expand_bounded(function, sources, expr, loc, scope, need);
                }
                Some(false) => {}
            }
        }
        if recursive {
            let mut vals = Vec::new();
            let mut missing = false;
            for source in &sources {
                match source {
                    ParamSource::Arg { expr, scope } => {
                        let val = match self.val(Query::Expr(*expr, *scope), need) {
                            Some(val) => Some(self.held_reads(*expr, val, *scope)?),
                            None => None,
                        };
                        missing |= val.is_none();
                        vals.push(val);
                    }
                    ParamSource::Default | ParamSource::Missing | ParamSource::Bound(_) => {
                        vals.push(None)
                    }
                }
            }
            if missing {
                return Ok(None);
            }
            if vals.iter().any(|val| matches!(val, Some(Val::Error))) {
                return Ok(Some(Val::Error));
            }
            let args = sources
                .iter()
                .zip(vals)
                .map(|(source, val)| match source {
                    ParamSource::Arg { .. } => val,
                    ParamSource::Default | ParamSource::Missing | ParamSource::Bound(_) => None,
                })
                .collect();
            return Ok(Some(self.scalar(
                Op::Recursive { function, args },
                loc,
                scope,
            )?));
        }
        let call_scope = match self.memo.call_scopes.get(&(expr, scope)) {
            Some(existing) => *existing,
            None => {
                if !self.within_inline_limit(function, &sources, loc, scope)? {
                    return Ok(Some(Val::Error));
                }
                let created = self.call_scope(function, sources, loc, scope, false)?;
                self.memo.call_scopes.insert((expr, scope), created);
                created
            }
        };
        self.inlined_value(decl.value, (expr, scope), call_scope, need)
    }

    fn expand_bounded(
        &mut self,
        function: FnId,
        sources: Vec<ParamSource>,
        expr: ExprId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let decl = self.hir.fn_decl(function)?;
        let Some(bound) = decl.bound.as_ref() else {
            return Err(Internal::missed("bound"));
        };
        let Some(limit) = bound.bound else {
            return Ok(Some(Val::Error));
        };
        let nesting = self.nesting_in(scope, function)?;
        let body = if nesting >= limit || self.memo.passed_over_calls.contains(&(expr, scope)) {
            bound.fallback
        } else {
            decl.value
        };
        let call_scope = match self.memo.call_scopes.get(&(expr, scope)) {
            Some(existing) => *existing,
            None => {
                let count = self.inline_count(function, &sources)?;
                if !self.charge_inlined(count, function, loc, scope)? {
                    return Ok(Some(Val::Error));
                }
                let created = self.call_scope(function, sources, loc, scope, false)?;
                self.deepen(created, function, nesting.saturating_add(1))?;
                self.memo.call_scopes.insert((expr, scope), created);
                created
            }
        };
        if nesting == 0 && !some!(self.fallback_agrees(function, call_scope, true, need)?) {
            return Ok(Some(Val::Error));
        }
        self.inlined_value(body, (expr, scope), call_scope, need)
    }

    fn inlined_value(
        &mut self,
        body: ExprId,
        (expr, scope): (ExprId, ScopeId),
        call_scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let fallback = self
            .hir
            .fn_decl(self.owner_of(call_scope)?)?
            .bound
            .as_ref()
            .is_some_and(|bound| bound.fallback == body);
        let ScopeKind::Locals(locals) = &self.scope_of(call_scope)?.kind else {
            return Err(Internal::missed("locals scope"));
        };
        let LocalsOwner::Fn(function) = locals.owner else {
            return Err(Internal::missed("function scope"));
        };
        let decl = self.hir.fn_decl(function)?;
        let mut locals_ready = true;
        for index in 0..decl.params.len() {
            locals_ready &= self
                .bound(Query::Param(call_scope, index_of(index)), need)
                .is_some();
        }
        for index in 0..decl.lets.len() {
            if fallback {
                break;
            }
            locals_ready &= self
                .bound(
                    Query::Let(
                        call_scope,
                        crate::semantics::analysis::let_ref::LetRef::Local(index_of(index)),
                    ),
                    need,
                )
                .is_some();
        }
        let value = self.val(Query::Expr(body, call_scope), need);
        if !locals_ready {
            return Ok(None);
        }
        if let Some(group) = self.memo.literal_groups.get(&(body, call_scope)).copied() {
            self.memo.literal_groups.insert((expr, scope), group);
        }
        Ok(value)
    }

    pub(in crate::semantics::analysis) fn siblings(
        &mut self,
        first: ExprId,
        second: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<(Option<Val>, Option<Val>), Internal> {
        let first_val = self.val(Query::Expr(first, scope), need);
        let sharing = self.same_call(first, second, scope)?;
        if sharing && first_val.is_none() {
            return Ok((None, None));
        }
        if sharing && self.memo.call_scopes.contains_key(&(first, scope)) {
            let shared = self.share_call(first, second, scope)?;
            let second_val = first_val.clone().filter(|_| shared).or(Some(Val::Error));
            return Ok((first_val, second_val));
        }
        let second_val = self.val(Query::Expr(second, scope), need);
        Ok((first_val, second_val))
    }

    fn same_call(
        &mut self,
        first: ExprId,
        second: ExprId,
        scope: ScopeId,
    ) -> Result<bool, Internal> {
        let hir = self.hir;
        let (ExprKind::Call(callee, args), ExprKind::Call(other_callee, other_args)) =
            (&hir.expr(first)?.kind, &hir.expr(second)?.kind)
        else {
            return Ok(false);
        };
        let pairs: Vec<(ExprId, ExprId)> = match (args, other_args) {
            (Args::Positional(values), Args::Positional(others))
                if values.len() == others.len() =>
            {
                values.iter().copied().zip(others.iter().copied()).collect()
            }
            (Args::Named(values), Args::Named(others))
                if values.len() == others.len()
                    && values
                        .iter()
                        .zip(others)
                        .all(|((name, _), (other_name, _))| name.text == other_name.text) =>
            {
                values
                    .iter()
                    .zip(others)
                    .map(|((_, value), (_, other))| (*value, *other))
                    .collect()
            }
            (Args::Positional(_) | Args::Named(_), Args::Positional(_) | Args::Named(_)) => {
                return Ok(false);
            }
        };
        let mut same_args = true;
        for (value, other) in pairs {
            if !self.same_path(value, other)? {
                same_args = false;
                break;
            }
        }
        if !same_args || !self.same_path(*callee, *other_callee)? {
            return Ok(false);
        }
        let ExprKind::Name(name) = &hir.expr(*callee)?.kind else {
            return Ok(false);
        };
        Ok(match self.find(name, scope)? {
            Found::Decl(Decl::Fn(function)) => !self.is_recursive(function)?,
            Found::Val(_)
            | Found::Query(_)
            | Found::Item(_)
            | Found::Decl(_)
            | Found::Placeholder
            | Found::Nothing => false,
        })
    }

    fn same_path(&self, first: ExprId, second: ExprId) -> Result<bool, Internal> {
        let mut pairs = vec![(first, second)];
        while let Some((left, right)) = pairs.pop() {
            match (&self.hir.expr(left)?.kind, &self.hir.expr(right)?.kind) {
                (ExprKind::Name(a), ExprKind::Name(b)) if a == b => {}
                (ExprKind::Field(a, name), ExprKind::Field(b, other))
                    if name.text == other.text =>
                {
                    pairs.push((*a, *b));
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    fn share_call(
        &mut self,
        first: ExprId,
        second: ExprId,
        scope: ScopeId,
    ) -> Result<bool, Internal> {
        if let Some(shared) = self.memo.call_scopes.get(&(second, scope)) {
            return Ok(self.memo.call_scopes.get(&(first, scope)) == Some(shared));
        }
        let body = self
            .memo
            .call_scopes
            .get(&(first, scope))
            .copied()
            .ok_or(Internal::missed("call scope"))?;
        let ScopeKind::Locals(locals) = &self.scope_of(body)?.kind else {
            return Err(Internal::missed("locals scope"));
        };
        let LocalsOwner::Fn(function) = locals.owner else {
            return Err(Internal::missed("function scope"));
        };
        let sources = locals.params.clone();
        let loc = self.hir.expr(second)?.loc;
        if !self.within_inline_limit(function, &sources, loc, scope)? {
            return Ok(false);
        }
        self.scope_of(body)?.chain.also(loc);
        self.memo.call_scopes.insert((second, scope), body);
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn within_inline_limit(
        &mut self,
        function: FnId,
        sources: &[ParamSource],
        loc: Loc,
        scope: ScopeId,
    ) -> Result<bool, Internal> {
        if self.inline_limit_reported {
            return Ok(false);
        }
        if self.inside_function_body(scope)? {
            return Ok(true);
        }
        let count = self.inline_count(function, sources)?;
        self.charge_inlined(count, function, loc, scope)
    }

    pub(in crate::semantics::analysis) fn charge_evaluated(
        &mut self,
        function: FnId,
        defaulted: &[usize],
        loc: Loc,
        scope: ScopeId,
    ) -> Result<(), Internal> {
        let mut count = 0u64;
        if self.evaluated_bodies.insert(function) {
            count = self.collected.inline.body(function)?;
        }
        for param in defaulted {
            if self.evaluated_defaults.insert((function, *param)) {
                count = count.saturating_add(self.collected.inline.default(function, *param)?);
            }
        }
        if count > 0 {
            self.charge_inlined(count, function, loc, scope)?;
        }
        Ok(())
    }

    pub(super) fn charge_inlined(
        &mut self,
        count: u64,
        function: FnId,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<bool, Internal> {
        if self.inline_limit_reported {
            return Ok(false);
        }
        self.inlined = self.inlined.saturating_add(count);
        if self.inlined <= self.limits.inlined_calls {
            return Ok(true);
        }
        self.report_inline_limit(function, loc, scope)?;
        Ok(false)
    }

    fn bounded_cycle(&self, function: FnId) -> Result<bool, Internal> {
        self.collected
            .bounded_cycle
            .get(function.index())
            .copied()
            .ok_or(Internal::missed("function"))
    }

    fn call_reads_cell(
        &mut self,
        function: FnId,
        sources: &[ParamSource],
        need: &mut Vec<Query>,
    ) -> Result<Option<bool>, Internal> {
        let mut reads = false;
        let mut missing = false;
        for source in sources {
            let ParamSource::Arg { expr, scope } = source else {
                continue;
            };
            let Some(val) = self.val(Query::Expr(*expr, *scope), need) else {
                missing = true;
                continue;
            };
            for node in val.nodes() {
                match self.sig(node, need) {
                    Some(sig) => reads |= sig.reactive,
                    None => missing = true,
                }
            }
        }
        if missing {
            return Ok(None);
        }
        if reads {
            return Ok(Some(true));
        }
        self.body_reads_cell(function, need)
    }

    pub(in crate::semantics::analysis) fn nesting_in(
        &self,
        scope: ScopeId,
        function: FnId,
    ) -> Result<u64, Internal> {
        Ok(self
            .nesting_of(scope)?
            .iter()
            .find(|(owner, _)| *owner == function)
            .map_or(0, |(_, depth)| *depth))
    }

    pub(in crate::semantics::analysis) fn in_expansion(
        &self,
        scope: ScopeId,
    ) -> Result<bool, Internal> {
        Ok(!self.nesting_of(scope)?.is_empty())
    }

    fn nesting_of(&self, scope: ScopeId) -> Result<Rc<Vec<(FnId, u64)>>, Internal> {
        let mut current = Some(scope);
        while let Some(id) = current {
            let record = self.scope_of(id)?;
            match &record.kind {
                ScopeKind::Locals(locals) => return Ok(locals.nesting.clone()),
                ScopeKind::ParamPrefix { locals, .. } => current = Some(*locals),
                ScopeKind::Global { .. } => break,
                ScopeKind::Body { .. }
                | ScopeKind::ForVar { .. }
                | ScopeKind::WireClause { .. }
                | ScopeKind::WrittenClause { .. } => {
                    current = record.parent;
                }
            }
        }
        Ok(Rc::new(Vec::new()))
    }

    fn owner_of(&self, scope: ScopeId) -> Result<FnId, Internal> {
        let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
            return Err(Internal::missed("locals scope"));
        };
        let LocalsOwner::Fn(function) = locals.owner else {
            return Err(Internal::missed("function scope"));
        };
        Ok(function)
    }

    fn deepen(&mut self, scope: ScopeId, function: FnId, depth: u64) -> Result<(), Internal> {
        let record = self
            .scopes
            .get_mut(scope.index())
            .ok_or(Internal::missed("scope"))?;
        let ScopeKind::Locals(locals) = &mut record.kind else {
            return Err(Internal::missed("locals scope"));
        };
        let mut nesting: Vec<(FnId, u64)> = locals
            .nesting
            .iter()
            .copied()
            .filter(|(owner, _)| *owner != function)
            .collect();
        nesting.push((function, depth));
        locals.nesting = Rc::new(nesting);
        Ok(())
    }

    pub(super) fn is_recursive(&self, function: FnId) -> Result<bool, Internal> {
        self.collected
            .recursive
            .get(function.index())
            .copied()
            .ok_or(Internal::missed("function"))
    }

    pub(super) fn inline_count(
        &self,
        function: FnId,
        sources: &[ParamSource],
    ) -> Result<u64, Internal> {
        self.collected.inline.call(function, &defaulted(sources))
    }

    pub(super) fn report_inline_limit(
        &mut self,
        function: FnId,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<(), Internal> {
        self.inline_limit_reported = true;
        let name = self
            .hir
            .fn_decl(function)?
            .name
            .as_ref()
            .map_or(String::new(), |name| name.text.clone());
        let limit = grouped(self.limits.inlined_calls);
        let help = self.default_calling(function, scope)?;
        self.stop(
            Code::InlineLimit,
            loc,
            Some(scope),
            (
                format!(
                    "this call of `{name}` makes the compilation inline more than {limit} calls"
                ),
                help,
            ),
        )
    }

    pub(in crate::semantics::analysis) fn default_calling(
        &self,
        function: FnId,
        scope: ScopeId,
    ) -> Result<Option<String>, Internal> {
        let ScopeKind::ParamPrefix { locals, count } = self.scope_of(scope)?.kind else {
            return Ok(None);
        };
        let ScopeKind::Locals(Locals {
            owner: LocalsOwner::Fn(owner),
            ..
        }) = self.scope_of(locals)?.kind
        else {
            return Ok(None);
        };
        if owner != function {
            return Ok(None);
        }
        let param = self
            .hir
            .fn_decl(function)?
            .params
            .get(count as usize)
            .and_then(|param| param.name.as_ref())
            .map(|name| name.text.clone())
            .ok_or(Internal::missed("parameter"))?;
        let name = self.function_name(function)?;
        Ok(Some(format!(
            "pass `{param}` in the call, or give it a default that does not call `{name}`, since `{param}`'s default makes every call of `{name}` another"
        )))
    }

    pub(super) fn inside_function_body(&self, scope: ScopeId) -> Result<bool, Internal> {
        let mut current = Some(scope);
        while let Some(id) = current {
            let record = self.scope_of(id)?;
            match &record.kind {
                ScopeKind::Locals(locals) => {
                    return Ok(matches!(locals.owner, LocalsOwner::Fn(_)));
                }
                ScopeKind::ParamPrefix { locals, .. } => current = Some(*locals),
                ScopeKind::Global { .. } => return Ok(false),
                ScopeKind::Body { .. }
                | ScopeKind::ForVar { .. }
                | ScopeKind::WireClause { .. }
                | ScopeKind::WrittenClause { .. } => {
                    current = record.parent;
                }
            }
        }
        Ok(false)
    }

    pub(in crate::semantics::analysis) fn call_scope(
        &mut self,
        function: FnId,
        params: Vec<ParamSource>,
        loc: Loc,
        caller: ScopeId,
        recursive: bool,
    ) -> Result<ScopeId, Internal> {
        let names = match self.memo.local_names.get(&function) {
            Some(names) => names.clone(),
            None => {
                let decl = self.hir.fn_decl(function)?;
                let mut names = BTreeMap::new();
                for (index, param) in decl.params.iter().enumerate() {
                    if let Some(name) = &param.name {
                        names.insert(name.text.clone(), LocalName::Param(index_of(index)));
                    }
                }
                for (index, local) in decl.lets.iter().enumerate() {
                    if let Some(name) = &local.name {
                        names
                            .entry(name.text.clone())
                            .or_insert(LocalName::Let(index_of(index)));
                    }
                }
                let names = Rc::new(names);
                self.memo.local_names.insert(function, names.clone());
                names
            }
        };
        let mut global = if self.hir.is_prelude_fn(function) {
            self.prelude_global
        } else {
            self.source_global
        };
        if self.in_name_probe(caller)? {
            global = self.names_global_of(global);
        }
        let inside_recursion = self.template_of(caller)?.is_some();
        let chain = if recursive && inside_recursion {
            self.chain(caller)?
        } else {
            self.chain(caller)?.push(loc, "in this call")
        };
        let template = if recursive {
            Some(ScopeId(crate::semantics::analysis::index::index(
                self.scopes.len(),
            )))
        } else {
            self.template_of(caller)?
        };
        let nesting = if recursive {
            Rc::new(Vec::new())
        } else {
            self.nesting_of(caller)?
        };
        let base = match self.template_of(caller)? {
            Some(outer) if recursive && inside_recursion => {
                self.memo.template_base.get(&outer).copied()
            }
            _ => None,
        };
        let base = match base {
            Some(base) => base,
            None if recursive => chain.depth(),
            None => 0,
        };
        let created = self.scope_in(
            ScopeKind::Locals(Locals {
                owner: LocalsOwner::Fn(function),
                params,
                names,
                recursive,
                nesting,
            }),
            Some(global),
            chain,
            template,
        )?;
        if recursive {
            self.memo.template_base.insert(created, base);
        }
        Ok(created)
    }

    pub(in crate::semantics::analysis) fn bind_arguments(
        &mut self,
        decl: &FnDecl,
        args: &Args,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Option<Vec<ParamSource>>, Internal> {
        let params = decl.params.as_slice();
        let function = decl.name.as_ref().map_or("", |name| name.text.as_str());
        let sources = match bound_sources(params, args, scope) {
            Ok(sources) => sources,
            Err(BindFault::TooMany { .. }) if !decl.skipped_params.is_empty() => {
                return Ok(None);
            }
            Err(BindFault::Unknown(name)) if decl.skipped_params.holds(&name.text) => {
                return Ok(None);
            }
            Err(BindFault::TooMany { given }) => {
                let count = params.len();
                self.error(
                    Code::ArgumentCount,
                    loc,
                    scope,
                    format!(
                        "`{function}` takes at most {count} argument{}, not {given}",
                        if count == 1 { "" } else { "s" }
                    ),
                )?;
                return Ok(None);
            }
            Err(BindFault::Unknown(name)) => {
                let declared: Vec<String> = params
                    .iter()
                    .filter_map(|param| param.name.as_ref())
                    .map(|param| format!("`{}`", param.text))
                    .collect();
                let help = match declared.as_slice() {
                    [] => format!(
                        "call `{function}` with no arguments, since it declares no parameters"
                    ),
                    _ => format!(
                        "name a parameter `{function}` declares: {}",
                        declared.join(", ")
                    ),
                };
                self.error_helped(
                    Code::UnknownParameter,
                    name.loc,
                    scope,
                    format!("the function has no parameter `{}`", name.text),
                    help,
                )?;
                return Ok(None);
            }
            Err(BindFault::Duplicate(name)) => {
                self.error(
                    Code::DuplicateArgument,
                    name.loc,
                    scope,
                    format!("the parameter `{}` is supplied twice", name.text),
                )?;
                return Ok(None);
            }
        };
        let mut complete = true;
        let required: Vec<&str> = params
            .iter()
            .filter(|param| param.default.is_none())
            .filter_map(|param| param.name.as_ref())
            .map(|name| name.text.as_str())
            .collect();
        for (param, source) in params.iter().zip(&sources) {
            if matches!(source, ParamSource::Missing) {
                if let Some(name) = &param.name {
                    self.error_helped(
                        Code::MissingArgument,
                        loc,
                        scope,
                        format!(
                            "the parameter `{}` has no default and no argument",
                            name.text
                        ),
                        format!(
                            "pass `{}` in the call, as in `{function}({})`",
                            name.text,
                            required.join(", ")
                        ),
                    )?;
                }
                complete = false;
            }
        }
        Ok(complete.then_some(sources))
    }

    pub(in crate::semantics::analysis) fn fallback_agrees(
        &mut self,
        function: FnId,
        scope: ScopeId,
        kinds: bool,
        need: &mut Vec<Query>,
    ) -> Result<Option<bool>, Internal> {
        let decl = self.hir.fn_decl(function)?;
        let Some(fallback) = decl.bound.as_ref().map(|bound| bound.fallback) else {
            return Ok(Some(true));
        };
        let body = decl.value;
        let body_type = some!(self.arm_type(body, scope, need)?);
        let fallback_type = some!(self.arm_type(fallback, scope, need)?);
        let kinds_agree = match (body_type.kind, fallback_type.kind) {
            (Some(body), Some(fallback)) => !kinds || unify_kinds(body, fallback).is_some(),
            _ => true,
        };
        let shapes_agree = match (&body_type.shape, &fallback_type.shape) {
            (ArmShape::Single, ArmShape::Array(_)) | (ArmShape::Array(_), ArmShape::Single) => {
                false
            }
            (ArmShape::Array(Some(body)), ArmShape::Array(Some(fallback))) => body == fallback,
            _ => true,
        };
        let message = match (kinds_agree, shapes_agree) {
            (true, true) => return Ok(Some(true)),
            (false, _) => {
                "the fallback of a bounded function and its body yield values of different types"
            }
            (true, false) => {
                "the fallback of a bounded function and its body hold arrays of different shapes"
            }
        };
        let loc = self.hir.expr(fallback)?.loc;
        self.error(Code::TypeMismatch, loc, scope, message)?;
        Ok(Some(false))
    }

    fn arm_type(
        &mut self,
        root: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<ArmType>, Internal> {
        let mut found: FastMap<(ExprId, ScopeId), Option<ArmType>> = FastMap::default();
        let mut parts: FastMap<(ExprId, ScopeId), Vec<(ExprId, ScopeId)>> = FastMap::default();
        let mut stack = vec![(root, scope, false)];
        while let Some((expr, scope, ready)) = stack.pop() {
            if found.contains_key(&(expr, scope)) {
                continue;
            }
            if !ready {
                let children = self.arm_parts(expr, scope)?;
                stack.push((expr, scope, true));
                stack.extend(children.iter().map(|(expr, scope)| (*expr, *scope, false)));
                parts.insert((expr, scope), children);
                continue;
            }
            let children = parts.remove(&(expr, scope)).unwrap_or_default();
            let recursion = self.calls_recursive(expr, scope)?;
            let tainted = recursion
                || children
                    .iter()
                    .any(|child| found.get(child).is_some_and(Option::is_some));
            let typed = if tainted {
                let mut types = Vec::with_capacity(children.len());
                for child in &children {
                    let known = match found.get(child).cloned().flatten() {
                        Some(known) => known,
                        None => some!(self.clean_arm_type(child.0, child.1, need)?),
                    };
                    types.push(known);
                }
                Some(if recursion {
                    ArmType::UNKNOWN
                } else {
                    self.combined_arm_type(expr, &types)?
                })
            } else {
                None
            };
            found.insert((expr, scope), typed);
        }
        match found.remove(&(root, scope)).flatten() {
            Some(typed) => Ok(Some(typed)),
            None => self.clean_arm_type(root, scope, need),
        }
    }

    fn arm_parts(
        &mut self,
        expr: ExprId,
        scope: ScopeId,
    ) -> Result<Vec<(ExprId, ScopeId)>, Internal> {
        let kind = self.hir.expr(expr)?.kind.clone();
        Ok(match kind {
            ExprKind::Name(name) => match self.find(&name, scope)? {
                Found::Query(Query::Let(owner, local)) => {
                    let value = self
                        .let_decl(owner, local)?
                        .map(|decl| decl.value)
                        .ok_or(Internal::missed("local let"))?;
                    vec![(value, self.let_scope(owner, local))]
                }
                Found::Query(Query::Param(owner, index)) => {
                    match self.param_default(owner, index)? {
                        Some(default) => vec![(default, self.prefix_scope(owner, index)?)],
                        None => Vec::new(),
                    }
                }
                Found::Val(_)
                | Found::Query(_)
                | Found::Item(_)
                | Found::Decl(_)
                | Found::Placeholder
                | Found::Nothing => Vec::new(),
            },
            ExprKind::Call(_, args) => args
                .values()
                .into_iter()
                .map(|value| (value, scope))
                .collect(),
            ExprKind::Match {
                arms, otherwise, ..
            } => arms
                .iter()
                .map(|arm| arm.value)
                .chain(otherwise)
                .map(|value| (value, scope))
                .collect(),
            other => other
                .children()
                .into_iter()
                .map(|child| (child, scope))
                .collect(),
        })
    }

    fn param_default(&self, scope: ScopeId, index: u32) -> Result<Option<ExprId>, Internal> {
        let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
            return Ok(None);
        };
        let (LocalsOwner::Fn(function), Some(ParamSource::Default)) =
            (locals.owner, locals.params.get(index as usize))
        else {
            return Ok(None);
        };
        Ok(self
            .hir
            .fn_decl(function)?
            .params
            .get(index as usize)
            .and_then(|param| param.default))
    }

    fn calls_recursive(&mut self, expr: ExprId, scope: ScopeId) -> Result<bool, Internal> {
        let ExprKind::Call(callee, _) = &self.hir.expr(expr)?.kind else {
            return Ok(false);
        };
        let ExprKind::Name(name) = &self.hir.expr(*callee)?.kind else {
            return Ok(false);
        };
        let name = name.clone();
        Ok(match self.find(&name, scope)? {
            Found::Decl(Decl::Fn(function)) => self.is_recursive(function)?,
            Found::Val(_)
            | Found::Query(_)
            | Found::Item(_)
            | Found::Decl(_)
            | Found::Placeholder
            | Found::Nothing => false,
        })
    }

    fn clean_arm_type(
        &mut self,
        expr: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<ArmType>, Internal> {
        let val = some!(self.val(Query::Expr(expr, scope), need));
        let known_kind = |kind: Kind| match kind {
            Kind::Int | Kind::Float | Kind::Bool | Kind::Enum(_) | Kind::Str | Kind::Node => {
                Some(kind)
            }
            Kind::Bare | Kind::Any | Kind::Error => None,
        };
        Ok(Some(match val {
            Val::Error => ArmType::UNKNOWN,
            Val::Array(array) => {
                let mut kind = None;
                for element in &array.elements {
                    let element_kind = known_kind(some!(self.sig(*element, need)).kind);
                    kind = unified(kind, element_kind);
                }
                let lengths = array.shape.lengths();
                let mut inner: Vec<u32> = lengths.iter().skip(1).copied().collect();
                inner.reverse();
                ArmType {
                    kind,
                    shape: ArmShape::Array((lengths != [0]).then_some(inner)),
                }
            }
            Val::Scalar(node) => {
                let kind = known_kind(some!(self.sig(node, need)).kind);
                let shape = if self.arrays.contains(&node) {
                    match some!(self.layout_of(node, need)?).shaped {
                        Ok(None) => ArmShape::Single,
                        Ok(Some(layout)) => {
                            let empty = layout.outer() == Some(0) && layout.inner().is_empty();
                            ArmShape::Array((!empty).then(|| layout.inner().to_vec()))
                        }
                        Err(_) => ArmShape::Unknown,
                    }
                } else {
                    ArmShape::Single
                };
                ArmType { kind, shape }
            }
        }))
    }

    fn combined_arm_type(&self, expr: ExprId, types: &[ArmType]) -> Result<ArmType, Internal> {
        let all_kinds = || {
            types
                .iter()
                .fold(None, |kind, typed| unified(kind, typed.kind))
        };
        Ok(match &self.hir.expr(expr)?.kind {
            ExprKind::If(..) => {
                let arms = types.get(1..).unwrap_or_default();
                ArmType {
                    kind: arms
                        .iter()
                        .fold(None, |kind, typed| unified(kind, typed.kind)),
                    shape: arms
                        .iter()
                        .fold(ArmShape::Unknown, |shape, typed| shape.meet(&typed.shape)),
                }
            }
            ExprKind::Match { .. } => ArmType {
                kind: all_kinds(),
                shape: types
                    .iter()
                    .fold(ArmShape::Unknown, |shape, typed| shape.meet(&typed.shape)),
            },
            ExprKind::Binary(op, ..) => match op {
                BinaryOp::Add => {
                    let array = types.iter().find_map(|typed| match &typed.shape {
                        ArmShape::Array(inner) => Some(inner.clone()),
                        ArmShape::Unknown | ArmShape::Single => None,
                    });
                    let single = types.iter().any(|typed| typed.shape == ArmShape::Single);
                    ArmType {
                        kind: all_kinds(),
                        shape: match array {
                            Some(inner) => ArmShape::Array(inner),
                            None if single => ArmShape::Single,
                            None => ArmShape::Unknown,
                        },
                    }
                }
                BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::IntDiv
                | BinaryOp::Rem
                | BinaryOp::Pow => ArmType {
                    kind: all_kinds(),
                    shape: ArmShape::Single,
                },
                BinaryOp::Div => ArmType {
                    kind: Some(Kind::Float),
                    shape: ArmShape::Single,
                },
                BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge
                | BinaryOp::Is
                | BinaryOp::IsNot
                | BinaryOp::And
                | BinaryOp::Or => ArmType {
                    kind: Some(Kind::Bool),
                    shape: ArmShape::Single,
                },
            },
            ExprKind::Unary(UnaryOp::Neg, _) => ArmType {
                kind: all_kinds(),
                shape: ArmShape::Single,
            },
            ExprKind::Unary(UnaryOp::Not, _) => ArmType {
                kind: Some(Kind::Bool),
                shape: ArmShape::Single,
            },
            ExprKind::Convert(target, _) => ArmType {
                kind: Some(if is_float(*target) {
                    Kind::Float
                } else {
                    Kind::Int
                }),
                shape: ArmShape::Single,
            },
            ExprKind::Array(_) => ArmType {
                kind: all_kinds(),
                shape: ArmShape::Array(
                    types
                        .iter()
                        .all(|typed| typed.shape == ArmShape::Single)
                        .then(Vec::new),
                ),
            },
            ExprKind::Index(..) | ExprKind::IndexFromEnd(..) => {
                let base = types.first().cloned().unwrap_or(ArmType::UNKNOWN);
                ArmType {
                    kind: base.kind,
                    shape: match base.shape {
                        ArmShape::Array(Some(mut inner)) => match inner.pop() {
                            Some(_) => ArmShape::Array(Some(inner)),
                            None => ArmShape::Single,
                        },
                        ArmShape::Array(None) | ArmShape::Unknown | ArmShape::Single => {
                            ArmShape::Unknown
                        }
                    },
                }
            }
            ExprKind::Slice { .. } | ExprKind::Name(_) => {
                types.first().cloned().unwrap_or(ArmType::UNKNOWN)
            }
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Field(..)
            | ExprKind::History { .. }
            | ExprKind::Call(..) => ArmType::UNKNOWN,
        })
    }
}

#[derive(Clone, PartialEq)]
enum ArmShape {
    Unknown,
    Single,
    Array(Option<Vec<u32>>),
}

impl ArmShape {
    fn meet(&self, other: &ArmShape) -> ArmShape {
        match (self, other) {
            (ArmShape::Unknown, known) | (known, ArmShape::Unknown) => known.clone(),
            (ArmShape::Array(Some(a)), ArmShape::Array(Some(b))) if a != b => ArmShape::Unknown,
            (ArmShape::Array(a), ArmShape::Array(b)) => ArmShape::Array(a.clone().or(b.clone())),
            (ArmShape::Single, ArmShape::Single) => ArmShape::Single,
            (ArmShape::Single, ArmShape::Array(_)) | (ArmShape::Array(_), ArmShape::Single) => {
                ArmShape::Unknown
            }
        }
    }
}

#[derive(Clone)]
struct ArmType {
    kind: Option<Kind>,
    shape: ArmShape,
}

impl ArmType {
    const UNKNOWN: ArmType = ArmType {
        kind: None,
        shape: ArmShape::Unknown,
    };
}

fn unified(current: Option<Kind>, other: Option<Kind>) -> Option<Kind> {
    match (current, other) {
        (Some(current), Some(other)) => unify_kinds(current, other),
        (None, other) => other,
        (current, None) => current,
    }
}

fn builtin_arity(builtin: Builtin) -> usize {
    match builtin {
        Builtin::Abs
        | Builtin::Floor
        | Builtin::Ceil
        | Builtin::Round
        | Builtin::Sqrt
        | Builtin::Sin
        | Builtin::Cos
        | Builtin::Tan
        | Builtin::Exp
        | Builtin::Log
        | Builtin::Len
        | Builtin::Bytes => 1,
        Builtin::Min | Builtin::Max | Builtin::Rand => 2,
        Builtin::Clamp | Builtin::Lerp => 3,
    }
}

fn index_of(position: usize) -> u32 {
    u32::try_from(position).unwrap_or(u32::MAX)
}
