use super::bound_sources::bound_sources;
use super::calls::INLINE_LIMIT;
use super::defaulted::defaulted;
use super::found::Found;
use crate::semantics::Internal;
use crate::semantics::analysis::attr_value_source::AttrValueSource;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::ids::{InstanceId, ScopeId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::collection::{BuiltinName, Decl};
use crate::semantics::ir::{Args, ExprId, ExprKind, FnId, Top};

enum Called {
    Function(FnId, Vec<usize>),
    Evaluated,
    Unresolved,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Reached {
    Body(FnId),
    Default(FnId, usize),
}

struct Counted {
    bodies: FastSet<FnId>,
    defaults: FastSet<(FnId, usize)>,
    reached: FastSet<Reached>,
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn count_inlining(&mut self) -> Result<(), Internal> {
        if self.inline_limit_reported {
            return Ok(());
        }
        let hir = self.hir;
        let mut total = self.inlined;
        let mut counted = Counted {
            bodies: self.evaluated_bodies.clone(),
            defaults: self.evaluated_defaults.clone(),
            reached: FastSet::default(),
        };
        for (root, scope) in self.value_roots()? {
            if self.inside_function_body(scope)? {
                continue;
            }
            let mut stack = vec![root];
            while let Some(expr) = stack.pop() {
                if self.memo.exprs.contains_key(&(expr, scope)) {
                    continue;
                }
                let node = hir.expr(expr)?;
                let ExprKind::Call(callee, args) = &node.kind else {
                    stack.extend(node.kind.children().into_iter().rev());
                    continue;
                };
                let (function, defaulted) = match self.called(*callee, args, scope)? {
                    Called::Function(function, defaulted) => (function, defaulted),
                    Called::Evaluated => {
                        stack.extend(args.values().into_iter().rev());
                        continue;
                    }
                    Called::Unresolved => continue,
                };
                let inlined = if self.is_recursive(function)? {
                    0
                } else {
                    self.collected.inline.call(function, &defaulted)?
                };
                total = total.saturating_add(inlined).saturating_add(self.reached(
                    function,
                    &defaulted,
                    &mut counted,
                )?);
                if total > INLINE_LIMIT {
                    return self.report_inline_limit(function, node.loc, scope);
                }
                stack.extend(args.values().into_iter().rev());
            }
        }
        Ok(())
    }

    fn called(&mut self, callee: ExprId, args: &Args, scope: ScopeId) -> Result<Called, Internal> {
        let ExprKind::Name(name) = &self.hir.expr(callee)?.kind else {
            return Ok(Called::Unresolved);
        };
        Ok(match self.find(name, scope)? {
            Found::Decl(Decl::Builtin(BuiltinName::Function(_))) => match args {
                Args::Positional(_) => Called::Evaluated,
                Args::Named(_) => Called::Unresolved,
            },
            Found::Decl(Decl::Fn(function)) => {
                let params = &self.hir.fn_decl(function)?.params;
                let Ok(sources) = bound_sources(params, args, scope) else {
                    return Ok(Called::Unresolved);
                };
                if sources
                    .iter()
                    .any(|source| matches!(source, ParamSource::Missing))
                {
                    Called::Unresolved
                } else {
                    Called::Function(function, defaulted(&sources))
                }
            }
            Found::Val(_) | Found::Query(_) | Found::Item(_) | Found::Decl(_) | Found::Nothing => {
                Called::Unresolved
            }
        })
    }

    fn reached(
        &self,
        start: FnId,
        defaulted: &[usize],
        counted: &mut Counted,
    ) -> Result<u64, Internal> {
        let inline = &self.collected.inline;
        let mut count = 0u64;
        let mut stack = vec![Reached::Body(start)];
        stack.extend(
            defaulted
                .iter()
                .map(|param| Reached::Default(start, *param)),
        );
        while let Some(reached) = stack.pop() {
            if !counted.reached.insert(reached) {
                continue;
            }
            let sites = match reached {
                Reached::Body(function) => {
                    if self.is_recursive(function)? && counted.bodies.insert(function) {
                        count = count.saturating_add(inline.body(function)?);
                    }
                    inline.body_sites(function)?
                }
                Reached::Default(function, param) => {
                    if self.is_recursive(function)? && counted.defaults.insert((function, param)) {
                        count = count.saturating_add(inline.default(function, param)?);
                    }
                    inline.default_sites(function, param)?
                }
            };
            for site in sites {
                stack.push(Reached::Body(site.callee));
                stack.extend(
                    site.defaulted
                        .iter()
                        .map(|param| Reached::Default(site.callee, *param)),
                );
            }
        }
        Ok(count)
    }

    fn value_roots(&mut self) -> Result<Vec<(ExprId, ScopeId)>, Internal> {
        let hir = self.hir;
        let global = self.source_global;
        let mut roots = Vec::new();
        for (position, top) in hir.top.iter().enumerate() {
            if hir.is_prelude_top(position) {
                continue;
            }
            match *top {
                Top::Input(input) => roots.push((hir.input(input)?.value, global)),
                Top::Let(id) => roots.push((hir.let_decl(id)?.value, global)),
                Top::Fn(_) | Top::Pattern(_) | Top::Placement(_) => {}
            }
        }
        for index in 0..self.instances.len() {
            let id = InstanceId(u32::try_from(index).map_err(|_| Internal::missed("instance"))?);
            self.attr_roots(ItemRef::Instance(id), &mut roots)?;
            let record = self.instance(id)?;
            if let Some(gate) = hir.placement(record.placement)?.gate {
                roots.push((gate, record.scope));
            }
        }
        for index in 0..self.wires.len() {
            let id = WireId(u32::try_from(index).map_err(|_| Internal::missed("wire"))?);
            let record = self.wire(id)?;
            let decl = hir.wire(record.wire)?;
            let (scope, clause_scope) = (record.scope, record.clause_scope);
            roots.push((decl.destination, scope));
            self.attr_roots(ItemRef::Wire(id), &mut roots)?;
            if let Some(gate) = decl.gate {
                roots.push((gate, clause_scope));
            }
        }
        roots.extend(self.gates.iter().copied());
        for index in 0..self.scopes.len() {
            let id = ScopeId(u32::try_from(index).map_err(|_| Internal::missed("scope"))?);
            let ScopeKind::Locals(locals) = &self.scope_of(id)?.kind else {
                continue;
            };
            let (owner, sources) = (locals.owner, locals.params.clone());
            let (params, lets) = match owner {
                crate::semantics::analysis::locals_owner::LocalsOwner::Fn(function) => {
                    let decl = hir.fn_decl(function)?;
                    (&decl.params, decl.lets.len())
                }
                crate::semantics::analysis::locals_owner::LocalsOwner::Pattern(pattern) => {
                    let decl = hir.pattern(pattern)?;
                    (&decl.params, decl.lets.len())
                }
            };
            for (position, source) in sources.iter().enumerate() {
                match source {
                    ParamSource::Arg { expr, scope } => roots.push((*expr, *scope)),
                    ParamSource::Default => {
                        let default = params
                            .get(position)
                            .and_then(|param| param.default)
                            .ok_or(Internal::missed("parameter default"))?;
                        if !self.inside_function_body(id)? {
                            let count = u32::try_from(position)
                                .map_err(|_| Internal::missed("parameter"))?;
                            roots.push((default, self.prefix_scope(id, count)?));
                        }
                    }
                    ParamSource::Missing | ParamSource::Bound(_) => {}
                }
            }
            for index in 0..lets {
                let local =
                    LetRef::Local(u32::try_from(index).map_err(|_| Internal::missed("local let"))?);
                let decl = self
                    .let_decl(id, local)?
                    .ok_or(Internal::missed("local let"))?;
                roots.push((decl.value, id));
            }
        }
        Ok(roots)
    }

    fn attr_roots(
        &self,
        item: ItemRef,
        roots: &mut Vec<(ExprId, ScopeId)>,
    ) -> Result<(), Internal> {
        for source in self.attr_sources(item)? {
            if self.attr_decl(item, source.attr)?.is_none() {
                continue;
            }
            if let AttrValueSource::Expr(expr, scope) = source.value {
                roots.push((expr, scope));
            }
        }
        Ok(())
    }
}
