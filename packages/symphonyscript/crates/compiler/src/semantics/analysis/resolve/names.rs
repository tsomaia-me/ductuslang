use super::found::Found;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::local_name::LocalName;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::collection::{BuiltinName, Decl};
use crate::semantics::ir::Loc;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn find(
        &mut self,
        name: &str,
        scope: ScopeId,
    ) -> Result<Found, Internal> {
        let mut current = Some(scope);
        let mut skip_locals: Option<ScopeId> = None;
        while let Some(id) = current {
            let parent = self.scope_of(id)?.parent;
            match &self.scope_of(id)?.kind {
                ScopeKind::ForVar { name: var, value } if var == name => {
                    return Ok(Found::Val(value.clone()));
                }
                ScopeKind::Body { names } => {
                    if let Some(item) = names.get(name) {
                        return Ok(Found::Item(*item));
                    }
                }
                ScopeKind::Locals(locals) if skip_locals != Some(id) => {
                    match locals.names.get(name) {
                        Some(LocalName::Param(index)) => {
                            return Ok(Found::Query(Query::Param(id, *index)));
                        }
                        Some(LocalName::Let(index)) => {
                            return Ok(Found::Query(Query::Let(id, LetRef::Local(*index))));
                        }
                        None => {}
                    }
                }
                ScopeKind::ParamPrefix { locals, count } => {
                    let (locals, count) = (*locals, *count);
                    if let ScopeKind::Locals(owner) = &self.scope_of(locals)?.kind
                        && let Some(LocalName::Param(index)) = owner.names.get(name)
                        && *index < count
                    {
                        return Ok(Found::Query(Query::Param(locals, *index)));
                    }
                    skip_locals = Some(locals);
                }
                ScopeKind::Global { prelude } => {
                    let namespace = if *prelude {
                        &self.collected.prelude
                    } else {
                        &self.collected.source
                    };
                    return Ok(match namespace.get(name) {
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
                    });
                }
                ScopeKind::ForVar { .. } | ScopeKind::Locals(_) | ScopeKind::WireClause { .. } => {}
            }
            current = parent;
        }
        Ok(Found::Nothing)
    }

    pub(in crate::semantics::analysis) fn unplaced_variant(
        &mut self,
        node: NodeId,
    ) -> Result<(), Internal> {
        let (loc, scope) = {
            let record = self.node(node)?;
            (record.loc, record.scope)
        };
        let name = match &self.node(node)?.op {
            Op::Bare(name) => Some(name.clone()),
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Read(_)
            | Op::Use(_)
            | Op::Arg(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::If(..)
            | Op::Match { .. }
            | Op::Pick { .. }
            | Op::Element { .. }
            | Op::Array(_)
            | Op::Convert(..)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::ToRead { .. } => None,
        };
        let message = self.unplaced_message(name.as_deref())?;
        let unless_values = self.untypable_values(node)?;
        self.error_unless_values(Code::UnknownName, loc, scope, message, unless_values)
    }

    pub(in crate::semantics::analysis) fn unplaced_variant_at(
        &mut self,
        name: Option<&str>,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<(), Internal> {
        let message = self.unplaced_message(name)?;
        self.error(Code::UnknownName, loc, scope, message)
    }

    fn unplaced_message(&self, name: Option<&str>) -> Result<String, Internal> {
        Ok(match name {
            Some(name) => {
                let owner = match self.variants.owner(name) {
                    Some(id) => self.enum_decl(id)?.name.as_str(),
                    None => "",
                };
                format!(
                    "`{name}` names a variant only where `{owner}` is expected; elsewhere it is written `{owner}::{name}`"
                )
            }
            None => "this value names a variant, and nothing here expects an enum".to_string(),
        })
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
    ) -> Result<Option<Val>, Internal> {
        Ok(match self.find(name, scope)? {
            Found::Val(val) => Some(val),
            Found::Query(query) => {
                let bound = some!(self.bound(query, need));
                Some(self.read(&bound, loc, scope)?)
            }
            Found::Item(item) => Some(self.scalar(Op::NodeRef(item), loc, scope)?),
            Found::Decl(Decl::Builtin(BuiltinName::Pi)) => Some(self.scalar(Op::Pi, loc, scope)?),
            Found::Decl(decl) => {
                let what = match decl {
                    Decl::Fn(_) | Decl::Builtin(_) => "a function, which is only called",
                    Decl::Pattern(_) => "a pattern, which is only placed",
                    Decl::Node(_) => "a node kind, which is only placed",
                    Decl::Wire(_) => "a wire kind, which is only placed",
                    Decl::Enum(_) => "an enum, whose variants are written `Enum::Variant`",
                    Decl::Input(_) | Decl::Let(_) | Decl::Instance(_) => "not a value",
                };
                self.error(Code::NotAValue, loc, scope, format!("`{name}` is {what}"))?;
                Some(Val::Error)
            }
            Found::Nothing => {
                if self.variants.owner(name).is_some() {
                    Some(self.scalar(Op::Bare(name.to_string()), loc, scope)?)
                } else {
                    Some(Val::Error)
                }
            }
        })
    }
}
