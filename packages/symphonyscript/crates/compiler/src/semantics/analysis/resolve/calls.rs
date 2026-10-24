use std::collections::BTreeMap;
use std::rc::Rc;

use symphonyscript_core::Builtin;

use super::found::Found;
use crate::diagnostics::Code;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::local_name::LocalName;
use crate::semantics::analysis::locals::Locals;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::val::Val;
use crate::semantics::collection::{BuiltinName, Decl};
use crate::semantics::ir::{Args, ExprId, ExprKind, FnId, Loc, Param};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn call(
        &mut self,
        callee: ExprId,
        args: &Args,
        expr: ExprId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        let callee_expr = self.hir.expr(callee);
        let ExprKind::Name(name) = &callee_expr.kind else {
            if !matches!(callee_expr.kind, ExprKind::Error) {
                self.error(
                    Code::NotCallable,
                    callee_expr.loc,
                    scope,
                    "only a function or a built-in is called",
                );
            }
            return Some(Val::Error);
        };
        match self.find(name, scope) {
            Found::Decl(Decl::Builtin(BuiltinName::Function(builtin))) => {
                self.builtin_call(builtin, args, loc, scope, need)
            }
            Found::Decl(Decl::Fn(function)) => {
                self.function_call(function, args, expr, loc, scope, need)
            }
            Found::Nothing => {
                if self.variants.owner(name).is_some() {
                    self.unplaced_variant_at(Some(name), callee_expr.loc, scope);
                }
                Some(Val::Error)
            }
            Found::Val(_) | Found::Query(_) | Found::Item(_) | Found::Decl(_) => {
                self.error(
                    Code::NotCallable,
                    callee_expr.loc,
                    scope,
                    format!("`{name}` is not a function"),
                );
                Some(Val::Error)
            }
        }
    }

    fn builtin_call(
        &mut self,
        builtin: Builtin,
        args: &Args,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        let Args::Positional(values) = args else {
            self.error(
                Code::NamedBuiltinArguments,
                loc,
                scope,
                format!(
                    "the built-in `{}` takes positional arguments only",
                    builtin.name()
                ),
            );
            return Some(Val::Error);
        };
        let vals: Vec<Option<Val>> = values
            .iter()
            .map(|value| self.val(Query::Expr(*value, scope), need))
            .collect();
        if vals.iter().any(Option::is_none) {
            return None;
        }
        let arity = builtin_arity(builtin);
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
            );
            return Some(Val::Error);
        }
        let mut operands = Vec::new();
        for val in vals.into_iter().flatten() {
            match val {
                Val::Scalar(node) => operands.push(node),
                Val::Array(_) => {
                    self.error(
                        Code::ArrayOperand,
                        loc,
                        scope,
                        "built-ins apply to single values, not arrays",
                    );
                    return Some(Val::Error);
                }
                Val::Error => return Some(Val::Error),
            }
        }
        Some(self.scalar(Op::Builtin(builtin, operands), loc, scope))
    }

    fn function_call(
        &mut self,
        function: FnId,
        args: &Args,
        expr: ExprId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        let decl = self.hir.fn_decl(function);
        let Some(sources) = self.bind_arguments(&decl.params, args, loc, scope) else {
            return Some(Val::Error);
        };
        let recursive = self
            .collected
            .recursive
            .get(function.index())
            .copied()
            .unwrap_or(false);
        if recursive {
            let mut vals = Vec::new();
            let mut missing = false;
            for source in &sources {
                match source {
                    ParamSource::Arg { expr, scope } => {
                        let val = self.val(Query::Expr(*expr, *scope), need);
                        missing |= val.is_none();
                        vals.push(val);
                    }
                    ParamSource::Default | ParamSource::Missing | ParamSource::Bound(_) => {
                        vals.push(None)
                    }
                }
            }
            if missing {
                return None;
            }
            let args = sources
                .iter()
                .zip(vals)
                .map(|(source, val)| match source {
                    ParamSource::Arg { .. } => val,
                    ParamSource::Default | ParamSource::Missing | ParamSource::Bound(_) => None,
                })
                .collect();
            return Some(self.scalar(Op::Recursive { function, args }, loc, scope));
        }
        let call_scope = match self.memo.call_scopes.get(&(expr, scope)) {
            Some(existing) => *existing,
            None => {
                if !self.within_inline_limit(function, &sources, loc, scope) {
                    return Some(Val::Error);
                }
                let created = self.call_scope(function, sources, loc, scope, false);
                self.memo.call_scopes.insert((expr, scope), created);
                created
            }
        };
        let mut locals_ready = true;
        for index in 0..decl.params.len() {
            locals_ready &= self
                .bound(Query::Param(call_scope, index_of(index)), need)
                .is_some();
        }
        for index in 0..decl.lets.len() {
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
        let value = self.val(Query::Expr(decl.value, call_scope), need);
        if !locals_ready {
            return None;
        }
        value
    }

    pub(in crate::semantics::analysis) fn siblings(
        &mut self,
        first: ExprId,
        second: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> (Option<Val>, Option<Val>) {
        let first_val = self.val(Query::Expr(first, scope), need);
        let sharing = self.same_call(first, second, scope);
        if sharing && first_val.is_none() {
            return (None, None);
        }
        if sharing && self.memo.call_scopes.contains_key(&(first, scope)) {
            let shared = self.share_call(first, second, scope);
            let second_val = first_val.clone().filter(|_| shared).or(Some(Val::Error));
            return (first_val, second_val);
        }
        let second_val = self.val(Query::Expr(second, scope), need);
        (first_val, second_val)
    }

    fn same_call(&mut self, first: ExprId, second: ExprId, scope: ScopeId) -> bool {
        let hir = self.hir;
        let (ExprKind::Call(callee, args), ExprKind::Call(other_callee, other_args)) =
            (&hir.expr(first).kind, &hir.expr(second).kind)
        else {
            return false;
        };
        let same_args = match (args, other_args) {
            (Args::Positional(values), Args::Positional(others)) => {
                values.len() == others.len()
                    && values
                        .iter()
                        .zip(others)
                        .all(|(value, other)| self.same_path(*value, *other))
            }
            (Args::Named(values), Args::Named(others)) => {
                values.len() == others.len()
                    && values
                        .iter()
                        .zip(others)
                        .all(|((name, value), (other_name, other))| {
                            name.text == other_name.text && self.same_path(*value, *other)
                        })
            }
            (Args::Positional(_), Args::Named(_)) | (Args::Named(_), Args::Positional(_)) => false,
        };
        if !same_args || !self.same_path(*callee, *other_callee) {
            return false;
        }
        let ExprKind::Name(name) = &hir.expr(*callee).kind else {
            return false;
        };
        match self.find(name, scope) {
            Found::Decl(Decl::Fn(function)) => !self
                .collected
                .recursive
                .get(function.index())
                .copied()
                .unwrap_or(true),
            Found::Val(_) | Found::Query(_) | Found::Item(_) | Found::Decl(_) | Found::Nothing => {
                false
            }
        }
    }

    fn same_path(&self, first: ExprId, second: ExprId) -> bool {
        let mut pairs = vec![(first, second)];
        while let Some((left, right)) = pairs.pop() {
            match (&self.hir.expr(left).kind, &self.hir.expr(right).kind) {
                (ExprKind::Name(a), ExprKind::Name(b)) if a == b => {}
                (ExprKind::Field(a, name), ExprKind::Field(b, other))
                    if name.text == other.text =>
                {
                    pairs.push((*a, *b));
                }
                _ => return false,
            }
        }
        true
    }

    fn share_call(&mut self, first: ExprId, second: ExprId, scope: ScopeId) -> bool {
        if let Some(shared) = self.memo.call_scopes.get(&(second, scope)) {
            return self.memo.call_scopes.get(&(first, scope)) == Some(shared);
        }
        let Some(body) = self.memo.call_scopes.get(&(first, scope)).copied() else {
            return false;
        };
        let ScopeKind::Locals(locals) = &self.scope_of(body).kind else {
            return false;
        };
        let LocalsOwner::Fn(function) = locals.owner else {
            return false;
        };
        let sources = locals.params.clone();
        let loc = self.hir.expr(second).loc;
        if !self.within_inline_limit(function, &sources, loc, scope) {
            return false;
        }
        self.scope_of(body).chain.also(loc);
        self.memo.call_scopes.insert((second, scope), body);
        true
    }

    pub(in crate::semantics::analysis) fn within_inline_limit(
        &mut self,
        function: FnId,
        sources: &[ParamSource],
        loc: Loc,
        scope: ScopeId,
    ) -> bool {
        if self.inline_limit_reported {
            return false;
        }
        if self.inside_inlined_body(scope) {
            return true;
        }
        let defaulted: Vec<usize> = sources
            .iter()
            .enumerate()
            .filter(|(_, source)| matches!(source, ParamSource::Default))
            .map(|(position, _)| position)
            .collect();
        let count = self.collected.inline.call(function, &defaulted);
        self.inlined = self.inlined.saturating_add(count);
        if self.inlined <= INLINE_LIMIT {
            return true;
        }
        self.inline_limit_reported = true;
        let name = self
            .hir
            .fn_decl(function)
            .name
            .as_ref()
            .map_or(String::new(), |name| name.text.clone());
        self.error(
            Code::InlineLimit,
            loc,
            scope,
            format!("this call of `{name}` makes the compilation inline more than 1,000,000 calls"),
        );
        false
    }

    fn inside_inlined_body(&self, scope: ScopeId) -> bool {
        let mut current = Some(scope);
        while let Some(id) = current {
            let record = self.scope_of(id);
            match &record.kind {
                ScopeKind::Locals(locals) => {
                    return matches!(locals.owner, LocalsOwner::Fn(_)) && !locals.recursive;
                }
                ScopeKind::ParamPrefix { locals, .. } => current = Some(*locals),
                ScopeKind::Global { .. } => return false,
                ScopeKind::Body { .. }
                | ScopeKind::ForVar { .. }
                | ScopeKind::WireClause { .. } => {
                    current = record.parent;
                }
            }
        }
        false
    }

    pub(in crate::semantics::analysis) fn call_scope(
        &mut self,
        function: FnId,
        params: Vec<ParamSource>,
        loc: Loc,
        caller: ScopeId,
        recursive: bool,
    ) -> ScopeId {
        let names = match self.memo.local_names.get(&function) {
            Some(names) => names.clone(),
            None => {
                let decl = self.hir.fn_decl(function);
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
        let global = if self.hir.is_prelude_fn(function) {
            self.prelude_global
        } else {
            self.source_global
        };
        let inside_recursion = self.template_of(caller).is_some();
        let chain = if recursive && inside_recursion {
            self.chain(caller)
        } else {
            self.chain(caller).push(loc)
        };
        let template = if recursive {
            Some(ScopeId(crate::semantics::analysis::index::index(
                self.scopes.len(),
            )))
        } else {
            self.template_of(caller)
        };
        self.scope_in(
            ScopeKind::Locals(Locals {
                owner: LocalsOwner::Fn(function),
                params,
                names,
                recursive,
            }),
            Some(global),
            chain,
            template,
        )
    }

    pub(in crate::semantics::analysis) fn bind_arguments(
        &mut self,
        params: &[Param],
        args: &Args,
        loc: Loc,
        scope: ScopeId,
    ) -> Option<Vec<ParamSource>> {
        let mut sources: Vec<Option<ParamSource>> = vec![None; params.len()];
        match args {
            Args::Positional(values) => {
                if values.len() > params.len() {
                    self.error(
                        Code::ArgumentCount,
                        loc,
                        scope,
                        format!(
                            "the function takes at most {} arguments, not {}",
                            params.len(),
                            values.len()
                        ),
                    );
                    return None;
                }
                for (slot, value) in sources.iter_mut().zip(values) {
                    *slot = Some(ParamSource::Arg {
                        expr: *value,
                        scope,
                    });
                }
            }
            Args::Named(values) => {
                for (name, value) in values {
                    let Some(position) = params.iter().position(|param| {
                        param.name.as_ref().is_some_and(|own| own.text == name.text)
                    }) else {
                        self.error(
                            Code::UnknownParameter,
                            name.loc,
                            scope,
                            format!("the function has no parameter `{}`", name.text),
                        );
                        return None;
                    };
                    if let Some(slot) = sources.get_mut(position) {
                        if slot.is_some() {
                            self.error(
                                Code::DuplicateArgument,
                                name.loc,
                                scope,
                                format!("the parameter `{}` is supplied twice", name.text),
                            );
                            return None;
                        }
                        *slot = Some(ParamSource::Arg {
                            expr: *value,
                            scope,
                        });
                    }
                }
            }
        }
        let mut bound = Vec::new();
        let mut complete = true;
        for (param, slot) in params.iter().zip(sources) {
            match slot {
                Some(source) => bound.push(source),
                None if param.default.is_some() => bound.push(ParamSource::Default),
                None => {
                    let name = param.name.as_ref().map_or("", |name| name.text.as_str());
                    self.error(
                        Code::MissingArgument,
                        loc,
                        scope,
                        format!("the parameter `{name}` has no default and no argument"),
                    );
                    complete = false;
                    bound.push(ParamSource::Missing);
                }
            }
        }
        complete.then_some(bound)
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
        | Builtin::Log => 1,
        Builtin::Min | Builtin::Max | Builtin::Rand => 2,
        Builtin::Clamp | Builtin::Lerp => 3,
    }
}

const INLINE_LIMIT: u64 = 1_000_000;

fn index_of(position: usize) -> u32 {
    u32::try_from(position).unwrap_or(u32::MAX)
}
