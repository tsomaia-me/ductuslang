use std::collections::BTreeSet;

use crate::semantics::Internal;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::chain::Chain;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::index::index;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::collection::Decl;
use crate::semantics::ir::{
    Attr, AttrValue, BlockId, BodyItem, ExprId, ExprKind, FnId, Hir, Iterable, LetId, Loc, Param,
    PatternId, PlacementId,
};

impl Engine<'_> {
    pub(crate) fn resolve_prelude_lets(&mut self) -> Result<(), Internal> {
        let global = self.prelude_names_global;
        for index in 0..self.hir.lets.len() {
            let id = LetId(u32::try_from(index).map_err(|_| Internal::missed("let"))?);
            if self.hir.is_prelude_let(id) {
                self.demand(Query::Let(global, LetRef::Top(id)))?;
            }
        }
        self.unexpanded_names(true)
    }

    pub(in crate::semantics::analysis) fn name_let(&mut self, id: LetId) -> Result<(), Internal> {
        self.demand(Query::Let(self.names_global, LetRef::Top(id)))?;
        if self.memo.cyclic_lets.contains(&id) {
            self.demand(Query::Let(self.source_global, LetRef::Top(id)))?;
        }
        Ok(())
    }

    pub(in crate::semantics::analysis) fn unused_lets(&mut self) -> Result<(), Internal> {
        let global = self.source_global;
        for position in 0..self.hir.lets.len() {
            let id = LetId(u32::try_from(position).map_err(|_| Internal::missed("let"))?);
            if !self.hir.is_prelude_let(id)
                && !self.memo.lets.contains_key(&(global, LetRef::Top(id)))
                && self.let_reads_cell(id)?
            {
                self.demand(Query::Let(global, LetRef::Top(id)))?;
            }
        }
        Ok(())
    }

    fn let_reads_cell(&self, root: LetId) -> Result<bool, Internal> {
        let hir = self.hir;
        let namespace = &self.collected.source;
        let mut lets = vec![root];
        let mut functions = Vec::new();
        let mut seen_lets = FastSet::default();
        let mut seen_functions = FastSet::default();
        loop {
            let (mut stack, locals) = if let Some(id) = lets.pop() {
                if !seen_lets.insert(id) || hir.is_prelude_let(id) {
                    continue;
                }
                let Some(decl) = self.let_decl(self.source_global, LetRef::Top(id))? else {
                    continue;
                };
                (vec![decl.value], BTreeSet::new())
            } else if let Some(function) = functions.pop() {
                if !seen_functions.insert(function) || hir.is_prelude_fn(function) {
                    continue;
                }
                let decl = hir.fn_decl(function)?;
                let locals: BTreeSet<&str> = decl
                    .params
                    .iter()
                    .filter_map(|param| param.name.as_ref())
                    .chain(decl.lets.iter().filter_map(|local| local.name.as_ref()))
                    .map(|name| name.text.as_str())
                    .collect();
                let mut stack: Vec<ExprId> = decl
                    .params
                    .iter()
                    .filter_map(|param| param.default)
                    .collect();
                stack.extend(decl.lets.iter().map(|local| local.value));
                stack.push(decl.value);
                stack.extend(decl.bound.iter().map(|bound| bound.fallback));
                (stack, locals)
            } else {
                return Ok(false);
            };
            while let Some(expr) = stack.pop() {
                let kind = &hir.expr(expr)?.kind;
                match kind {
                    ExprKind::Name(name) if !locals.contains(name.as_str()) => {
                        match namespace.get(name) {
                            Some(Decl::Input(_) | Decl::State(_)) => return Ok(true),
                            Some(Decl::Let(id)) => lets.push(id),
                            Some(Decl::Fn(callee)) => functions.push(callee),
                            Some(_) | None => {}
                        }
                    }
                    ExprKind::Field(..) => return Ok(true),
                    _ => {}
                }
                stack.extend(kind.children());
            }
        }
    }

    pub(in crate::semantics::analysis) fn unexpanded_names(
        &mut self,
        prelude: bool,
    ) -> Result<(), Internal> {
        let hir = self.hir;
        let global = if prelude {
            self.prelude_names_global
        } else {
            self.names_global
        };
        let mut scanned = 0;
        let mut patterns = FastSet::default();
        self.resolved_owners(&mut scanned, &mut patterns)?;
        for position in 0..hir.patterns.len() {
            let pattern = PatternId(index(position));
            if hir.is_prelude_pattern(pattern) != prelude || patterns.contains(&pattern) {
                continue;
            }
            self.resolve_pattern_names(pattern)?;
            self.resolved_owners(&mut scanned, &mut patterns)?;
        }
        let mut called = vec![false; hir.fns.len()];
        for (caller, sites) in self.collected.calls.iter().enumerate() {
            for (callee, _) in sites {
                if *callee != caller
                    && let Some(flag) = called.get_mut(*callee)
                {
                    *flag = true;
                }
            }
        }
        for roots_only in [true, false] {
            for position in 0..hir.fns.len() {
                let function = FnId(index(position));
                if hir.is_prelude_fn(function) != prelude
                    || (roots_only && called.get(position).copied().unwrap_or(false))
                {
                    continue;
                }
                self.resolved_owners(&mut scanned, &mut patterns)?;
                if self.memo.named_bodies.contains(&function) {
                    continue;
                }
                let loc = hir.fn_decl(function)?.loc;
                let scope = self.named_scope(function, loc, global)?;
                self.scopes
                    .get_mut(scope.index())
                    .ok_or(Internal::missed("scope"))?
                    .chain = Chain::default();
                for query in self.body_queries(function, scope)? {
                    self.demand(query)?;
                }
            }
        }
        Ok(())
    }

    fn resolved_owners(
        &mut self,
        scanned: &mut usize,
        patterns: &mut FastSet<PatternId>,
    ) -> Result<(), Internal> {
        while *scanned < self.scopes.len() {
            let scope = ScopeId(index(*scanned));
            *scanned += 1;
            let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
                continue;
            };
            let owner = locals.owner;
            if self.in_trial(scope)? {
                continue;
            }
            match owner {
                LocalsOwner::Fn(function) => self.memo.named_bodies.insert(function),
                LocalsOwner::Pattern(pattern) => patterns.insert(pattern),
            };
        }
        Ok(())
    }

    pub(in crate::semantics::analysis) fn named_function(
        &mut self,
        function: FnId,
        loc: Loc,
        caller: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<()>, Internal> {
        let scope = self.named_scope(function, loc, caller)?;
        for query in self.body_queries(function, scope)? {
            if self.in_progress(query) {
                continue;
            }
            let resolved = match query {
                Query::Expr(..) => self.val(query, need).is_some(),
                _ => self.bound(query, need).is_some(),
            };
            if !resolved {
                return Ok(None);
            }
        }
        Ok(Some(()))
    }

    fn named_scope(
        &mut self,
        function: FnId,
        loc: Loc,
        caller: ScopeId,
    ) -> Result<ScopeId, Internal> {
        if let Some(scope) = self.memo.named_scopes.get(&function) {
            return Ok(*scope);
        }
        let params = self
            .hir
            .fn_decl(function)?
            .params
            .iter()
            .map(|param| match param.default {
                Some(_) => ParamSource::Default,
                None => ParamSource::Bound(BoundVal::Error),
            })
            .collect();
        let scope = self.call_scope(function, params, loc, caller, false)?;
        self.memo.named_scopes.insert(function, scope);
        self.memo.named_bodies.insert(function);
        Ok(scope)
    }

    fn body_queries(&self, function: FnId, scope: ScopeId) -> Result<Vec<Query>, Internal> {
        let decl = self.hir.fn_decl(function)?;
        let mut queries = vec![Query::Expr(decl.value, scope)];
        for local in 0..decl.lets.len() {
            let local = u32::try_from(local).map_err(|_| Internal::missed("local let"))?;
            queries.push(Query::Let(scope, LetRef::Local(local)));
        }
        for (position, param) in decl.params.iter().enumerate() {
            if param.default.is_some() {
                let position =
                    u32::try_from(position).map_err(|_| Internal::missed("parameter"))?;
                queries.push(Query::Param(scope, position));
            }
        }
        if let Some(bound) = &decl.bound {
            queries.push(Query::Expr(bound.fallback, scope));
        }
        Ok(queries)
    }

    fn demand_defaults(&mut self, params: &[Param], scope: ScopeId) -> Result<(), Internal> {
        for (position, param) in params.iter().enumerate() {
            if let Some(default) = param.default {
                let count = u32::try_from(position).map_err(|_| Internal::missed("parameter"))?;
                let prefix = self.prefix_scope(scope, count)?;
                self.demand(Query::Expr(default, prefix))?;
            }
        }
        Ok(())
    }

    fn resolve_pattern_names(&mut self, pattern: PatternId) -> Result<(), Internal> {
        let hir = self.hir;
        let decl = hir.pattern(pattern)?;
        let params = decl
            .params
            .iter()
            .map(|_| ParamSource::Bound(BoundVal::Error))
            .collect();
        let scope = self.pattern_locals(pattern, params, Chain::default(), true)?;
        for local in 0..decl.lets.len() {
            let local = u32::try_from(local).map_err(|_| Internal::missed("local let"))?;
            self.demand(Query::Let(scope, LetRef::Local(local)))?;
        }
        self.demand_defaults(&decl.params, scope)?;
        let mut exprs = Vec::new();
        let mut blocks = vec![decl.body];
        while let Some(block) = blocks.pop() {
            for item in &hir.block(block)?.items {
                match item {
                    BodyItem::Placement(placement) => {
                        placement_exprs(hir, *placement, &mut exprs, &mut blocks)?;
                    }
                    BodyItem::Wire(wire) => {
                        let wire = hir.wire(*wire)?;
                        exprs.extend(wire.default.iter().chain(wire.gate.iter()));
                        exprs.extend(attr_exprs(&wire.attrs));
                        exprs.push(wire.destination);
                    }
                    BodyItem::Bundle { members, each, .. } => {
                        for member in members {
                            placement_exprs(hir, *member, &mut exprs, &mut blocks)?;
                        }
                        if let Some(each) = each {
                            exprs.extend(iterable_exprs(each.iterable));
                            for member in &each.members {
                                placement_exprs(hir, *member, &mut exprs, &mut blocks)?;
                            }
                        }
                    }
                    BodyItem::For { iterable, body, .. } => {
                        exprs.extend(iterable_exprs(*iterable));
                        blocks.push(*body);
                    }
                    BodyItem::When {
                        arms, otherwise, ..
                    } => {
                        for arm in arms {
                            exprs.push(arm.guard);
                            blocks.push(arm.body);
                        }
                        blocks.extend(otherwise.iter());
                    }
                }
            }
        }
        for expr in exprs {
            self.demand(Query::Expr(expr, scope))?;
        }
        Ok(())
    }

    pub(in crate::semantics::analysis) fn declaration(
        &self,
        query: Query,
    ) -> Result<Option<(String, Loc)>, Internal> {
        Ok(match query {
            Query::Let(scope, local) => {
                let decl = some!(self.let_decl(scope, local)?);
                let name = some!(decl.name.as_ref());
                Some((name.text.clone(), name.loc))
            }
            Query::Param(scope, index) => {
                let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
                    return Err(Internal::missed("locals scope"));
                };
                let params = match locals.owner {
                    LocalsOwner::Fn(function) => &self.hir.fn_decl(function)?.params,
                    LocalsOwner::Pattern(pattern) => &self.hir.pattern(pattern)?.params,
                };
                let param = params
                    .get(index as usize)
                    .ok_or(Internal::missed("parameter"))?;
                let name = some!(param.name.as_ref());
                Some((name.text.clone(), name.loc))
            }
            Query::Input(input) => {
                let name = some!(self.hir.input(input)?.name.as_ref());
                Some((name.text.clone(), name.loc))
            }
            Query::State(state) => {
                let name = some!(self.hir.state(state)?.name.as_ref());
                Some((name.text.clone(), name.loc))
            }
            Query::Attr(item, attr) => {
                let decl = some!(self.attr_decl(item, attr)?);
                let owner = match item {
                    ItemRef::Instance(id) => self.instance(id)?.name.clone(),
                    ItemRef::Wire(id) => self.wire(id)?.name.clone(),
                    ItemRef::Use(_) => None,
                };
                let source = self
                    .attr_sources(item)?
                    .iter()
                    .find(|source| source.attr == attr)
                    .map(|source| source.loc);
                let loc = match source {
                    Some(loc) => loc,
                    None => self.item_loc(item)?,
                };
                let text = match owner {
                    Some(owner) => format!("{owner}.{}", decl.name),
                    None => decl.name.clone(),
                };
                Some((text, loc))
            }
            Query::Expr(expr, scope) => {
                let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
                    return Ok(None);
                };
                let LocalsOwner::Fn(function) = locals.owner else {
                    return Ok(None);
                };
                let decl = self.hir.fn_decl(function)?;
                if decl.value != expr {
                    return Ok(None);
                }
                let name = some!(decl.name.as_ref());
                Some((name.text.clone(), name.loc))
            }
            Query::Destination(_) | Query::Sig(_) | Query::Eval(..) => None,
        })
    }
}

fn placement_exprs(
    hir: &Hir,
    placement: PlacementId,
    exprs: &mut Vec<ExprId>,
    blocks: &mut Vec<BlockId>,
) -> Result<(), Internal> {
    let placement = hir.placement(placement)?;
    exprs.extend(placement.default.iter().chain(placement.gate.iter()));
    exprs.extend(attr_exprs(&placement.attrs));
    blocks.extend(placement.body.iter());
    Ok(())
}

fn attr_exprs(attrs: &[Attr]) -> impl Iterator<Item = ExprId> + '_ {
    attrs.iter().filter_map(|attr| match attr.value {
        AttrValue::Expr(expr) => Some(expr),
        AttrValue::Set(_) => None,
    })
}

fn iterable_exprs(iterable: Iterable) -> Vec<ExprId> {
    match iterable {
        Iterable::Range(start, end) => vec![start, end],
        Iterable::Values(values) => vec![values],
    }
}
