use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::ir::{LetId, Loc};

impl Engine<'_> {
    pub(crate) fn resolve_prelude_lets(&mut self) -> Result<(), Internal> {
        for index in 0..self.hir.lets.len() {
            let id = LetId(u32::try_from(index).map_err(|_| Internal::missed("let"))?);
            if self.hir.is_prelude_let(id) {
                self.demand(Query::Let(self.prelude_global, LetRef::Top(id)))?;
            }
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
