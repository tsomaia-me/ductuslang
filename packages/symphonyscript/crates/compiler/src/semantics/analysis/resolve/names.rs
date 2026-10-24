use crate::diagnostics::Code;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::query::{LetRef, Query};
use crate::semantics::analysis::scope::{LocalName, ScopeKind};
use crate::semantics::analysis::val::Val;
use crate::semantics::collection::{BuiltinName, Decl};
use crate::semantics::ir::Loc;

pub(crate) enum Found {
    Val(Val),
    Query(Query),
    Item(ItemRef),
    Decl(Decl),
    Nothing,
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn find(&mut self, name: &str, scope: ScopeId) -> Found {
        let mut current = Some(scope);
        let mut skip_locals: Option<ScopeId> = None;
        while let Some(id) = current {
            let parent = self.scope_of(id).parent;
            match &self.scope_of(id).kind {
                ScopeKind::ForVar { name: var, value } if var == name => {
                    return Found::Val(value.clone());
                }
                ScopeKind::Body { names } => {
                    if let Some(item) = names.get(name) {
                        return Found::Item(*item);
                    }
                }
                ScopeKind::Locals(locals) if skip_locals != Some(id) => {
                    match locals.names.get(name) {
                        Some(LocalName::Param(index)) => {
                            return Found::Query(Query::Param(id, *index));
                        }
                        Some(LocalName::Let(index)) => {
                            return Found::Query(Query::Let(id, LetRef::Local(*index)));
                        }
                        None => {}
                    }
                }
                ScopeKind::ParamPrefix { locals, count } => {
                    let (locals, count) = (*locals, *count);
                    if let ScopeKind::Locals(owner) = &self.scope_of(locals).kind
                        && let Some(LocalName::Param(index)) = owner.names.get(name)
                        && *index < count
                    {
                        return Found::Query(Query::Param(locals, *index));
                    }
                    skip_locals = Some(locals);
                }
                ScopeKind::Global { prelude } => {
                    let namespace = if *prelude {
                        &self.collected.prelude
                    } else {
                        &self.collected.source
                    };
                    return match namespace.get(name) {
                        Some(Decl::Input(input)) => Found::Query(Query::Input(input)),
                        Some(Decl::Let(id)) => {
                            let global = self.global_of_let(id);
                            Found::Query(Query::Let(global, LetRef::Top(id)))
                        }
                        Some(Decl::Instance(placement)) => match self.top_instances.get(&placement)
                        {
                            Some(instance) => Found::Item(ItemRef::Instance(*instance)),
                            None => Found::Val(Val::Error),
                        },
                        Some(decl) => Found::Decl(decl),
                        None => Found::Nothing,
                    };
                }
                _ => {}
            }
            current = parent;
        }
        Found::Nothing
    }

    pub(in crate::semantics::analysis) fn global_of_let(
        &self,
        id: crate::semantics::ir::LetId,
    ) -> ScopeId {
        if self.hir.is_prelude_let(id) {
            self.prelude_global
        } else {
            self.source_global
        }
    }

    pub(in crate::semantics::analysis) fn lookup(
        &mut self,
        name: &str,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        match self.find(name, scope) {
            Found::Val(val) => Some(val),
            Found::Query(query) => {
                let bound = self.bound(query, need)?;
                Some(self.read(&bound, loc, scope))
            }
            Found::Item(item) => Some(self.scalar(Op::NodeRef(item), loc, scope)),
            Found::Decl(Decl::Builtin(BuiltinName::Pi)) => Some(self.scalar(Op::Pi, loc, scope)),
            Found::Decl(decl) => {
                let what = match decl {
                    Decl::Fn(_) | Decl::Builtin(_) => "a function, which is only called",
                    Decl::Pattern(_) => "a pattern, which is only placed",
                    Decl::Node(_) => "a node kind, which is only placed",
                    Decl::Wire(_) => "a wire kind, which is only placed",
                    Decl::Enum(_) => "an enum, whose variants are written `Enum::Variant`",
                    _ => "not a value",
                };
                self.error(Code::NotAValue, loc, scope, format!("`{name}` is {what}"));
                Some(Val::Error)
            }
            Found::Nothing => {
                if crate::semantics::collection::variant_owner(self.vocabulary, name).is_some() {
                    Some(self.scalar(Op::Bare(name.to_string()), loc, scope))
                } else {
                    Some(Val::Error)
                }
            }
        }
    }
}
