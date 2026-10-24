use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope::{LocalsOwner, ScopeKind};
use crate::semantics::ir::Loc;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn declaration(
        &self,
        query: Query,
    ) -> Option<(String, Loc)> {
        match query {
            Query::Let(scope, local) => {
                let decl = self.let_decl(scope, local)?;
                let name = decl.name.as_ref()?;
                Some((name.text.clone(), name.loc))
            }
            Query::Param(scope, index) => {
                let ScopeKind::Locals(locals) = &self.scope_of(scope).kind else {
                    return None;
                };
                let params = match locals.owner {
                    LocalsOwner::Fn(function) => &self.hir.fn_decl(function).params,
                    LocalsOwner::Pattern(pattern) => &self.hir.pattern(pattern).params,
                };
                let name = params.get(index as usize)?.name.as_ref()?;
                Some((name.text.clone(), name.loc))
            }
            Query::Input(input) => {
                let name = self.hir.input(input).name.as_ref()?;
                Some((name.text.clone(), name.loc))
            }
            Query::Attr(item, attr) => {
                let decl = self.attr_decl(item, attr)?;
                let owner = match item {
                    ItemRef::Instance(id) => {
                        self.instance(id).and_then(|record| record.name.clone())
                    }
                    ItemRef::Wire(id) => self.wire(id).and_then(|record| record.name.clone()),
                    ItemRef::Use(_) => None,
                };
                let loc = self
                    .attr_sources(item)
                    .iter()
                    .find(|source| source.attr == attr)
                    .map_or_else(|| self.item_loc(item), |source| source.loc);
                let text = match owner {
                    Some(owner) => format!("{owner}.{}", decl.name),
                    None => decl.name.clone(),
                };
                Some((text, loc))
            }
            Query::Expr(expr, scope) => {
                let ScopeKind::Locals(locals) = &self.scope_of(scope).kind else {
                    return None;
                };
                let LocalsOwner::Fn(function) = locals.owner else {
                    return None;
                };
                let decl = self.hir.fn_decl(function);
                if decl.value != expr {
                    return None;
                }
                let name = decl.name.as_ref()?;
                Some((name.text.clone(), name.loc))
            }
            Query::Destination(_) | Query::Sig(_) | Query::Eval(..) => None,
        }
    }
}
