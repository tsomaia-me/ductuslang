use std::collections::BTreeMap;
use std::rc::Rc;

use symphonyscript_core::{AttrId, NodeKind, Segment, WireKind};

use super::context::Context;
use super::item_kind::ItemKind;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::attr_source::AttrSource;
use crate::semantics::analysis::attr_value_source::AttrValueSource;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::chain::Chain;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{InstanceId, ScopeId, UseId, WireId};
use crate::semantics::analysis::index::index;
use crate::semantics::analysis::instance::Instance;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::local_name::LocalName;
use crate::semantics::analysis::locals::Locals;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::path_parent::PathParent;
use crate::semantics::analysis::pattern_use::PatternUse;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::wire::Wire;
use crate::semantics::collection::{Decl, StandIn};
use crate::semantics::ir::{AttrValue, Loc, PatternId, PlacementId, WireId as HirWireId};
use crate::semantics::reporting::grouped;

impl<'a> Engine<'a> {
    pub(in crate::semantics::analysis) fn instance(
        &self,
        id: InstanceId,
    ) -> Result<&Instance, Internal> {
        self.instances
            .get(id.index())
            .ok_or(Internal::missed("instance"))
    }

    pub(in crate::semantics::analysis) fn wire(&self, id: WireId) -> Result<&Wire, Internal> {
        self.wires.get(id.index()).ok_or(Internal::missed("wire"))
    }

    pub(in crate::semantics::analysis) fn pattern_use(
        &self,
        id: UseId,
    ) -> Result<&PatternUse, Internal> {
        self.uses
            .get(id.index())
            .ok_or(Internal::missed("pattern use"))
    }

    pub(in crate::semantics::analysis) fn instance_mut(
        &mut self,
        id: InstanceId,
    ) -> Result<&mut Instance, Internal> {
        self.instances
            .get_mut(id.index())
            .ok_or(Internal::missed("instance"))
    }

    pub(in crate::semantics::analysis) fn wire_mut(
        &mut self,
        id: WireId,
    ) -> Result<&mut Wire, Internal> {
        self.wires
            .get_mut(id.index())
            .ok_or(Internal::missed("wire"))
    }

    pub(in crate::semantics::analysis) fn pattern_use_mut(
        &mut self,
        id: UseId,
    ) -> Result<&mut PatternUse, Internal> {
        self.uses
            .get_mut(id.index())
            .ok_or(Internal::missed("pattern use"))
    }

    pub(in crate::semantics::analysis) fn kind_decl(
        &self,
        scope: ScopeId,
        name: &str,
    ) -> Result<Option<Decl>, Internal> {
        let mut current = Some(scope);
        while let Some(id) = current {
            let scope = self.scope_of(id)?;
            if let ScopeKind::Global { prelude } = scope.kind {
                let namespace = if prelude {
                    &self.collected.prelude
                } else {
                    &self.collected.source
                };
                return Ok(namespace.get(name));
            }
            current = scope.parent;
        }
        Ok(None)
    }

    pub(in crate::semantics::analysis) fn count_item(
        &mut self,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<bool, Internal> {
        if self.limit_reached {
            return Ok(false);
        }
        self.expanded += 1;
        let bound = self.limits.expanded_items.min(self.id_space);
        if self.expanded > bound {
            self.limit_reached = true;
            let message = format!(
                "the expansion yields more than {} instances and wires",
                grouped(bound)
            );
            self.stop(Code::ExpansionLimit, loc, Some(scope), (message, None))?;
            return Ok(false);
        }
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn count_step(
        &mut self,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<bool, Internal> {
        if self.limit_reached {
            return Ok(false);
        }
        self.steps += 1;
        let bound = self.limits.expansion_steps.min(self.id_space);
        if self.steps > bound {
            self.limit_reached = true;
            let message = format!(
                "the expansion runs more than {} `for` iterations and pattern uses",
                grouped(bound)
            );
            self.stop(Code::ExpansionLimit, loc, Some(scope), (message, None))?;
            return Ok(false);
        }
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn segment(
        &mut self,
        parent: Option<PathParent>,
        name: Option<&str>,
        kind_name: &'a str,
    ) -> Segment {
        if let Some(name) = name {
            return Segment::Named(name.to_string());
        }
        let Some(parent) = parent else {
            return Segment::Named(kind_name.to_string());
        };
        let counter = self.counters.entry((parent, kind_name)).or_insert(0);
        let index = *counter;
        *counter += 1;
        Segment::Anonymous {
            name: kind_name.to_string(),
            index,
        }
    }

    pub(in crate::semantics::analysis) fn new_instance(
        &mut self,
        kind: NodeKind,
        placement: PlacementId,
        scope: ScopeId,
        name: Option<String>,
        segment: Option<Segment>,
        context: &Context,
    ) -> Result<Option<InstanceId>, Internal> {
        let loc = self.hir.placement(placement)?.loc;
        if !self.count_item(loc, scope)? {
            return Ok(None);
        }
        self.claim_id(self.instances.len(), "instances", loc)?;
        let id = InstanceId(index(self.instances.len()));
        let attrs = self.placement_attrs(ItemKind::Node(kind), placement, scope)?;
        self.instances.push(Instance {
            kind,
            placement,
            scope,
            name,
            segment,
            path_parent: context.path_parent,
            owner: context.owner,
            uses: context.uses.clone(),
            conditions: context.conditions,
            attrs,
            sequence: Vec::new(),
            body_scope: None,
            loc,
            expanded: false,
        });
        Ok(Some(id))
    }

    pub(in crate::semantics::analysis) fn new_wire(
        &mut self,
        kind: WireKind,
        wire: HirWireId,
        scope: ScopeId,
        segment: Option<Segment>,
        source: InstanceId,
        context: &Context,
    ) -> Result<Option<WireId>, Internal> {
        let decl = self.hir.wire(wire)?;
        let loc = decl.loc;
        if !self.count_item(loc, scope)? {
            return Ok(None);
        }
        self.claim_id(self.wires.len(), "wires", loc)?;
        let id = WireId(index(self.wires.len()));
        let chain = self.chain(scope)?;
        let clause_scope = self.scope(ScopeKind::WireClause { wire: id }, Some(scope), chain)?;
        let attrs = self.wire_attrs(kind, wire, clause_scope)?;
        self.wires.push(Wire {
            kind,
            wire,
            scope,
            clause_scope,
            name: decl.name.as_ref().map(|name| name.text.clone()),
            segment,
            path_parent: context.path_parent,
            source,
            uses: context.uses.clone(),
            conditions: context.conditions,
            attrs,
            loc,
        });
        Ok(Some(id))
    }

    pub(in crate::semantics::analysis) fn new_use(
        &mut self,
        pattern: PatternId,
        placement: PlacementId,
        caller: ScopeId,
        naming: (Option<String>, Option<Segment>),
        context: &Context,
    ) -> Result<Option<UseId>, Internal> {
        let (name, segment) = naming;
        let owner = some!(context.owner);
        let loc = self.hir.placement(placement)?.loc;
        if !self.count_step(loc, caller)? {
            return Ok(None);
        }
        let locals = self.use_locals(pattern, placement, caller)?;
        self.claim_id(self.uses.len(), "pattern uses", loc)?;
        let id = UseId(index(self.uses.len()));
        self.uses
            .push(crate::semantics::analysis::pattern_use::PatternUse {
                pattern,
                placement,
                caller,
                locals,
                body: None,
                name,
                segment,
                path_parent: context.path_parent,
                owner,
                uses: context.uses.clone(),
                conditions: context.conditions,
                loc,
            });
        Ok(Some(id))
    }

    pub(in crate::semantics::analysis) fn use_locals(
        &mut self,
        pattern: PatternId,
        placement: PlacementId,
        caller: ScopeId,
    ) -> Result<ScopeId, Internal> {
        let decl = self.hir.pattern(pattern)?;
        let site = self.hir.placement(placement)?;
        let mut sources: Vec<Option<ParamSource>> = vec![None; decl.params.len()];
        if let Some((_, loc)) = site.flags.first() {
            self.error(
                Code::UseForm,
                *loc,
                caller,
                "a pattern use carries no flags",
            )?;
        }
        if site.body.is_some() {
            self.error(
                Code::UseForm,
                site.loc,
                caller,
                "a pattern use carries no body",
            )?;
        }
        if let Some(default) = site.default {
            match decl.params.iter().position(|param| param.is_default) {
                Some(position) => {
                    *sources
                        .get_mut(position)
                        .ok_or(Internal::missed("parameter"))? = Some(ParamSource::Arg {
                        expr: default,
                        scope: caller,
                    });
                }
                None if decl.skipped_default => {}
                None => {
                    let loc = self.hir.expr(default)?.loc;
                    self.error(
                        Code::NoDefaultParameter,
                        loc,
                        caller,
                        "`/expr` supplies the parameter marked `default`, and this pattern has none",
                    )?;
                }
            }
        }
        for attr in &site.attrs {
            let Some(position) = decl.params.iter().position(|param| {
                param
                    .name
                    .as_ref()
                    .is_some_and(|name| name.text == attr.name.text)
            }) else {
                if !decl.skipped_params.holds(&attr.name.text) {
                    self.error(
                        Code::UnknownParameter,
                        attr.name.loc,
                        caller,
                        format!("the pattern has no parameter `{}`", attr.name.text),
                    )?;
                }
                continue;
            };
            let source = match attr.value {
                AttrValue::Expr(expr) => ParamSource::Arg {
                    expr,
                    scope: caller,
                },
                AttrValue::Set(value) => {
                    let node = self.push_node(Op::Bool(value), attr.loc, caller)?;
                    ParamSource::Bound(BoundVal::Scalar(Bound::Value(node)))
                }
            };
            match sources
                .get_mut(position)
                .ok_or(Internal::missed("parameter"))?
            {
                slot @ None => *slot = Some(source),
                Some(_) => self.error(
                    Code::DuplicateArgument,
                    attr.name.loc,
                    caller,
                    format!("the parameter `{}` is supplied twice", attr.name.text),
                )?,
            }
        }
        let mut params = Vec::new();
        for (param, source) in decl.params.iter().zip(sources) {
            params.push(match source {
                Some(source) => source,
                None if param.default.is_some() => ParamSource::Default,
                None => {
                    if let Some(name) = &param.name {
                        self.error(
                            Code::MissingArgument,
                            site.loc,
                            caller,
                            format!(
                                "the parameter `{}` has no default and no argument",
                                name.text
                            ),
                        )?;
                    }
                    ParamSource::Missing
                }
            });
        }
        let chain = self
            .chain(caller)?
            .push(site.loc, "in this use of the pattern");
        self.pattern_locals(pattern, params, chain, false)
    }

    pub(in crate::semantics::analysis) fn pattern_locals(
        &mut self,
        pattern: PatternId,
        params: Vec<ParamSource>,
        chain: Chain,
        names_only: bool,
    ) -> Result<ScopeId, Internal> {
        let names = match self.memo.pattern_names.get(&pattern) {
            Some(names) => names.clone(),
            None => {
                let decl = self.hir.pattern(pattern)?;
                let mut names = BTreeMap::new();
                for (position, param) in decl.params.iter().enumerate() {
                    if let Some(name) = &param.name {
                        names.insert(name.text.clone(), LocalName::Param(index(position)));
                    }
                }
                for (position, local) in decl.lets.iter().enumerate() {
                    if let Some(name) = &local.name {
                        names
                            .entry(name.text.clone())
                            .or_insert(LocalName::Let(index(position)));
                    }
                }
                let names = Rc::new(names);
                self.memo.pattern_names.insert(pattern, names.clone());
                names
            }
        };
        let mut global = if self.hir.is_prelude_pattern(pattern) {
            self.prelude_global
        } else {
            self.source_global
        };
        if names_only {
            global = self.names_global_of(global);
        }
        self.scope(
            ScopeKind::Locals(Locals {
                owner: LocalsOwner::Pattern(pattern),
                params,
                names,
                recursive: false,
                nesting: std::rc::Rc::new(Vec::new()),
            }),
            Some(global),
            chain,
        )
    }

    pub(in crate::semantics::analysis) fn stand_in(
        &self,
        pattern: PatternId,
    ) -> Result<Option<StandIn>, Internal> {
        self.collected
            .stand_ins
            .get(pattern.index())
            .copied()
            .ok_or(Internal::missed("pattern"))
    }

    pub(in crate::semantics::analysis) fn stand_in_target(
        &self,
        mut pattern: PatternId,
    ) -> Result<Option<PlacementId>, Internal> {
        loop {
            let placement = match some!(self.stand_in(pattern)?) {
                StandIn::Node(placement) => return Ok(Some(placement)),
                StandIn::Use(placement) => placement,
            };
            let kind = some!(self.hir.placement(placement)?.kind.as_ref());
            let global = if self.hir.is_prelude_pattern(pattern) {
                self.prelude_global
            } else {
                self.source_global
            };
            match some!(self.kind_decl(global, &kind.text)?) {
                Decl::Pattern(next) if !self.is_cyclic(next)? => pattern = next,
                Decl::Node(_) => return Ok(Some(placement)),
                Decl::Input(_)
                | Decl::Let(_)
                | Decl::State(_)
                | Decl::Fn(_)
                | Decl::Native(_)
                | Decl::Pattern(_)
                | Decl::Instance(_)
                | Decl::Builtin(_)
                | Decl::Wire(_)
                | Decl::Enum(_) => return Ok(None),
            }
        }
    }

    pub(in crate::semantics::analysis) fn is_placeholder(
        &self,
        pattern: PatternId,
    ) -> Result<bool, Internal> {
        self.collected
            .placeholder_patterns
            .get(pattern.index())
            .copied()
            .ok_or(Internal::missed("pattern"))
    }

    pub(in crate::semantics::analysis) fn stands_for_nothing_known(
        &self,
        item: ItemRef,
    ) -> Result<bool, Internal> {
        match item {
            ItemRef::Use(id) => self.is_placeholder(self.pattern_use(id)?.pattern),
            ItemRef::Instance(_) | ItemRef::Wire(_) => Ok(false),
        }
    }

    pub(in crate::semantics::analysis) fn may_name(
        &mut self,
        item: ItemRef,
        name: &str,
    ) -> Result<bool, Internal> {
        if self.stands_for_nothing_known(item)? {
            return Ok(true);
        }
        let body = match item {
            ItemRef::Use(id) => self.hir.pattern(self.pattern_use(id)?.pattern)?.body,
            ItemRef::Instance(id) => {
                let placement = self.hir.placement(self.instance(id)?.placement)?;
                if placement.skipped_block.holds(name) {
                    return Ok(true);
                }
                let Some(body) = placement.body else {
                    return Ok(false);
                };
                body
            }
            ItemRef::Wire(_) => return Ok(false),
        };
        let unknown = match self.memo.unknown_bodies.get(&body) {
            Some(unknown) => unknown.clone(),
            None => {
                let unknown = Rc::new(self.hir.body_unknown(body)?);
                self.memo.unknown_bodies.insert(body, unknown.clone());
                unknown
            }
        };
        Ok(unknown.holds(name))
    }

    pub(in crate::semantics::analysis) fn is_cyclic(
        &self,
        pattern: PatternId,
    ) -> Result<bool, Internal> {
        self.collected
            .cyclic_patterns
            .get(pattern.index())
            .copied()
            .ok_or(Internal::missed("pattern"))
    }

    fn wire_attrs(
        &mut self,
        kind: WireKind,
        wire: HirWireId,
        scope: ScopeId,
    ) -> Result<Vec<AttrSource>, Internal> {
        let hir = self.hir;
        let decl = hir.wire(wire)?;
        self.attr_sources_of(
            ItemKind::Wire(kind),
            &decl.flags,
            decl.default,
            &decl.attrs,
            decl.loc,
            scope,
        )
    }

    fn placement_attrs(
        &mut self,
        kind: ItemKind,
        placement: PlacementId,
        scope: ScopeId,
    ) -> Result<Vec<AttrSource>, Internal> {
        let hir = self.hir;
        let decl = hir.placement(placement)?;
        self.attr_sources_of(
            kind,
            &decl.flags,
            decl.default,
            &decl.attrs,
            decl.loc,
            scope,
        )
    }

    fn attr_sources_of(
        &mut self,
        kind: ItemKind,
        flags: &[(char, Loc)],
        default: Option<crate::semantics::ir::ExprId>,
        attrs: &[crate::semantics::ir::Attr],
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Vec<AttrSource>, Internal> {
        let (kind_name, decls, default_attr) = match kind {
            ItemKind::Node(kind) => {
                let decl = self.node_decl(kind)?;
                (decl.name.as_str(), decl.attrs.as_slice(), decl.default_attr)
            }
            ItemKind::Wire(kind) => {
                let decl = self.wire_decl(kind)?;
                (decl.name.as_str(), decl.attrs.as_slice(), decl.default_attr)
            }
        };
        let mut sources: Vec<AttrSource> = Vec::new();
        let mut set = |engine: &mut Self, source: AttrSource| -> Result<(), Internal> {
            if let Some(earlier) = sources.iter().find(|existing| existing.attr == source.attr) {
                let name = decls
                    .get(source.attr.index())
                    .ok_or(Internal::missed("attribute"))?
                    .name
                    .as_str();
                let earlier = earlier.loc;
                engine.error_with_chain(
                    Code::DuplicateAttribute,
                    source.loc,
                    vec![(earlier, "first set here".to_string())],
                    scope,
                    format!("the attribute `{name}` is set twice"),
                )?;
                return Ok(());
            }
            sources.push(source);
            Ok(())
        };
        for (flag, flag_loc) in flags {
            match decls.iter().position(|decl| decl.flag == Some(*flag)) {
                Some(position) => set(
                    self,
                    AttrSource {
                        attr: AttrId(index(position)),
                        value: AttrValueSource::Set(true),
                        loc: *flag_loc,
                    },
                )?,
                None => self.error(
                    Code::UnknownFlag,
                    *flag_loc,
                    scope,
                    format!("`{kind_name}` has no attribute flagged `{flag}`"),
                )?,
            }
        }
        if let Some(default) = default {
            let default_loc = self.hir.expr(default)?.loc;
            match default_attr {
                Some(attr) => set(
                    self,
                    AttrSource {
                        attr,
                        value: AttrValueSource::Expr(default, scope),
                        loc: default_loc,
                    },
                )?,
                None => self.error(
                    Code::NoDefaultAttribute,
                    default_loc,
                    scope,
                    format!("`{kind_name}` has no default attribute for `/expr`"),
                )?,
            }
        }
        for attr in attrs {
            let Some((position, decl)) = decls
                .iter()
                .enumerate()
                .find(|(_, decl)| decl.name == attr.name.text)
            else {
                self.error(
                    Code::UnknownAttribute,
                    attr.name.loc,
                    scope,
                    format!("`{kind_name}` has no attribute `{}`", attr.name.text),
                )?;
                continue;
            };
            let value = match attr.value {
                AttrValue::Expr(expr) => AttrValueSource::Expr(expr, scope),
                AttrValue::Set(value) => {
                    if decl.ty != symphonyscript_core::Ty::BOOL {
                        self.error(
                            Code::TypeMismatch,
                            attr.loc,
                            scope,
                            format!(
                                "`{}` alone or with `!` sets a `bool` attribute, and `{}` is not one",
                                attr.name.text, attr.name.text
                            ),
                        )?;
                        continue;
                    }
                    AttrValueSource::Set(value)
                }
            };
            set(
                self,
                AttrSource {
                    attr: AttrId(index(position)),
                    value,
                    loc: attr.loc,
                },
            )?;
        }
        for (position, decl) in decls.iter().enumerate() {
            let attr = AttrId(index(position));
            if decl.default.is_none() && !sources.iter().any(|source| source.attr == attr) {
                self.error(
                    Code::MissingAttribute,
                    loc,
                    scope,
                    format!(
                        "`{kind_name}` needs a value for `{}`, which has no default",
                        decl.name
                    ),
                )?;
            }
        }
        Ok(sources)
    }

    pub(in crate::semantics::analysis) fn item_loc(&self, item: ItemRef) -> Result<Loc, Internal> {
        Ok(match item {
            ItemRef::Instance(id) => self.instance(id)?.loc,
            ItemRef::Wire(id) => self.wire(id)?.loc,
            ItemRef::Use(id) => self.pattern_use(id)?.loc,
        })
    }

    pub(in crate::semantics::analysis) fn item_scope(
        &self,
        item: ItemRef,
    ) -> Result<ScopeId, Internal> {
        Ok(match item {
            ItemRef::Instance(id) => self.instance(id)?.scope,
            ItemRef::Wire(id) => self.wire(id)?.scope,
            ItemRef::Use(id) => self.pattern_use(id)?.caller,
        })
    }

    pub(in crate::semantics::analysis) fn item_description(
        &self,
        item: ItemRef,
    ) -> Result<String, Internal> {
        Ok(match item {
            ItemRef::Instance(id) => {
                let instance = self.instance(id)?;
                let kind = self.node_decl(instance.kind)?.name.as_str();
                match &instance.name {
                    Some(name) => format!("the `{kind}` named `{name}`"),
                    None => format!("the `{kind}`"),
                }
            }
            ItemRef::Wire(id) => {
                let kind = self.wire_decl(self.wire(id)?.kind)?.name.as_str();
                format!("the `{kind}` wire")
            }
            ItemRef::Use(id) => {
                let pattern = self.pattern_use(id)?.pattern;
                let name = self
                    .hir
                    .pattern(pattern)?
                    .name
                    .as_ref()
                    .map_or("", |name| name.text.as_str());
                format!("the use of `{name}`")
            }
        })
    }
}
