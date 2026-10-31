use super::found::Found;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::bound_val::BoundVal;
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
use crate::semantics::ir::{ExprId, Loc};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn find(
        &mut self,
        name: &str,
        scope: ScopeId,
    ) -> Result<Found, Internal> {
        let mut current = Some(scope);
        let mut skip_locals: Option<ScopeId> = None;
        while let Some(id) = current {
            crate::semantics::counted_walk::CountedWalk::NamesFind.step();
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
                        Some(Decl::State(state)) => Found::Query(Query::State(state)),
                        Some(Decl::Let(declared)) => {
                            let mut global = self.global_of_let(declared);
                            if (id == self.names_global
                                || id == self.prelude_names_global
                                || self.in_name_probe(scope)?)
                                && !self
                                    .memo
                                    .lets
                                    .contains_key(&(global, LetRef::Top(declared)))
                            {
                                global = self.names_global_of(global);
                            }
                            Found::Query(Query::Let(global, LetRef::Top(declared)))
                        }
                        Some(Decl::Instance(placement)) => match self.top_instances.get(&placement)
                        {
                            Some(instance) => Found::Item(ItemRef::Instance(*instance)),
                            None => Found::Val(Val::Error),
                        },
                        Some(decl) => Found::Decl(decl),
                        None if namespace.is_duplicated(name) => Found::Placeholder,
                        None => Found::Nothing,
                    });
                }
                ScopeKind::ForVar { .. }
                | ScopeKind::Locals(_)
                | ScopeKind::WireClause { .. }
                | ScopeKind::WrittenClause { .. } => {}
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
        let mut source = node;
        while let Op::Chosen(inner) = self.node(source)?.op {
            source = inner;
        }
        let name = match &self.node(source)?.op {
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
            | Op::Chosen(_)
            | Op::Arg(_)
            | Op::Slot(_)
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
            | Op::For(_)
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::History { .. }
            | Op::Stepped(_)
            | Op::Native { .. } => None,
        };
        let (message, help) = self.unplaced_message(name.as_deref())?;
        let unless_values = self.untypable_values(node)?;
        self.error_unless_values(Code::UnknownName, loc, scope, message, help, unless_values)
    }

    pub(in crate::semantics::analysis) fn unplaced_placeholder(
        &mut self,
        node: NodeId,
    ) -> Result<(), Internal> {
        let (loc, scope) = {
            let record = self.node(node)?;
            (record.loc, record.scope)
        };
        let mut source = node;
        while let Op::Chosen(inner) | Op::Use(inner) = self.node(source)?.op {
            source = inner;
        }
        let name = match &self.node(source)?.op {
            Op::Bare(name) => Some(name.clone()),
            _ => None,
        };
        self.unplaced_variant_at(name.as_deref(), loc, scope)?;
        self.typing.untyped_values.insert(source);
        Ok(())
    }

    pub(in crate::semantics::analysis) fn unplaced_variant_at(
        &mut self,
        name: Option<&str>,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<(), Internal> {
        if self.in_name_probe(scope)? {
            return Ok(());
        }
        match self.unplaced_message(name)? {
            (message, Some(help)) => {
                self.error_helped(Code::UnknownName, loc, scope, message, help)
            }
            (message, None) => self.error(Code::UnknownName, loc, scope, message),
        }
    }

    fn unplaced_message(&self, name: Option<&str>) -> Result<(String, Option<String>), Internal> {
        Ok(match name {
            Some(name) => {
                let owner = match self.variants.owner(name) {
                    Some(id) => self.enum_decl(id)?.name.as_str(),
                    None => "",
                };
                (
                    format!(
                        "`{name}` names a variant only where `{owner}` is expected; elsewhere it is written `{owner}::{name}`"
                    ),
                    Some(format!(
                        "write `{owner}::{name}`, or use `{name}` in a position that expects a `{owner}`"
                    )),
                )
            }
            None => (
                "this value names a variant, and nothing here expects an enum".to_string(),
                None,
            ),
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
        expr: ExprId,
        name: &str,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        Ok(match self.find(name, scope)? {
            Found::Val(Val::Scalar(node)) => Some(self.scalar(Op::Chosen(node), loc, scope)?),
            Found::Val(val) => Some(val),
            Found::Query(query) => {
                let bound = some!(self.bound(query, need));
                let val = self.read(&bound, loc, scope)?;
                if let (BoundVal::Array(array), Query::Let(..) | Query::Param(..)) = (&bound, query)
                {
                    let group = self.bound_group(query, array, loc)?;
                    for read in val.nodes() {
                        self.memo.groups.insert(read, group);
                    }
                }
                if let Some(group) = self
                    .bound_value(query)?
                    .and_then(|value| self.memo.literal_groups.get(&value).copied())
                {
                    self.memo.literal_groups.insert((expr, scope), group);
                }
                Some(val)
            }
            Found::Item(item) => Some(self.scalar(Op::NodeRef(item), loc, scope)?),
            Found::Placeholder => Some(Val::Error),
            Found::Decl(Decl::Builtin(BuiltinName::Pi)) => Some(self.scalar(Op::Pi, loc, scope)?),
            Found::Decl(decl) => {
                let what = match decl {
                    Decl::Fn(_) | Decl::Native(_) | Decl::Builtin(_) => {
                        "a function, which is only called"
                    }
                    Decl::Pattern(_) => "a pattern, which is only placed",
                    Decl::Node(_) => "a node kind, which is only placed",
                    Decl::Wire(_) => "a wire kind, which is only placed",
                    Decl::Enum(_) => "an enum, whose variants are written `Enum::Variant`",
                    Decl::Input(_) | Decl::Let(_) | Decl::State(_) | Decl::Instance(_) => {
                        "not a value"
                    }
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
