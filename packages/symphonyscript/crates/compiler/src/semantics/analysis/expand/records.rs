use std::collections::BTreeMap;
use std::rc::Rc;

use symphonyscript_core::{AttrId, NodeKind, Segment, WireKind};

use crate::diagnostics::Code;
use crate::semantics::analysis::attr_source::{AttrSource, AttrValueSource};
use crate::semantics::analysis::bound::{Bound, BoundVal};
use crate::semantics::analysis::chain::Chain;
use crate::semantics::analysis::engine::{Engine, index};
use crate::semantics::analysis::ids::{ConditionsId, InstanceId, ScopeId, UseId, WireId};
use crate::semantics::analysis::instance::Instance;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::path_parent::PathParent;
use crate::semantics::analysis::pattern_use::PatternUse;
use crate::semantics::analysis::scope::{LocalName, Locals, LocalsOwner, ParamSource, ScopeKind};
use crate::semantics::analysis::wire::Wire;
use crate::semantics::collection::{Decl, StandIn};
use crate::semantics::ir::{AttrValue, Loc, PatternId, PlacementId, WireId as HirWireId};

pub(crate) const EXPANSION_LIMIT: usize = 1_000_000;

pub(crate) struct Context {
    pub owner: Option<InstanceId>,
    pub path_parent: Option<PathParent>,
    pub conditions: Option<ConditionsId>,
    pub uses: Chain,
}

impl<'a> Engine<'a> {
    pub(in crate::semantics::analysis) fn instance(&self, id: InstanceId) -> &Instance {
        &self.instances[id.index()]
    }

    pub(in crate::semantics::analysis) fn wire(&self, id: WireId) -> &Wire {
        &self.wires[id.index()]
    }

    pub(in crate::semantics::analysis) fn pattern_use(&self, id: UseId) -> &PatternUse {
        &self.uses[id.index()]
    }

    pub(in crate::semantics::analysis) fn kind_decl(
        &self,
        scope: ScopeId,
        name: &str,
    ) -> Option<Decl> {
        let mut current = Some(scope);
        while let Some(id) = current {
            let scope = self.scope_of(id);
            if let ScopeKind::Global { prelude } = scope.kind {
                let namespace = if prelude {
                    &self.collected.prelude
                } else {
                    &self.collected.source
                };
                return namespace.get(name);
            }
            current = scope.parent;
        }
        None
    }

    pub(in crate::semantics::analysis) fn count_item(&mut self, loc: Loc, scope: ScopeId) -> bool {
        if self.limit_reached {
            return false;
        }
        self.expanded += 1;
        if self.expanded > EXPANSION_LIMIT {
            self.limit_reached = true;
            self.error(
                Code::ExpansionLimit,
                loc,
                scope,
                "the expansion yields more than 1,000,000 instances and wires",
            );
            return false;
        }
        true
    }

    pub(in crate::semantics::analysis) fn count_step(&mut self, loc: Loc, scope: ScopeId) -> bool {
        if self.limit_reached {
            return false;
        }
        self.steps += 1;
        if self.steps > EXPANSION_LIMIT {
            self.limit_reached = true;
            self.error(
                Code::ExpansionLimit,
                loc,
                scope,
                "the expansion runs more than 1,000,000 `for` iterations and pattern uses",
            );
            return false;
        }
        true
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
    ) -> Option<InstanceId> {
        let loc = self.hir.placement(placement).loc;
        if !self.count_item(loc, scope) {
            return None;
        }
        let id = InstanceId(index(self.instances.len()));
        let attrs = self.placement_attrs(ItemKind::Node(kind), placement, scope);
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
        Some(id)
    }

    pub(in crate::semantics::analysis) fn new_wire(
        &mut self,
        kind: WireKind,
        wire: HirWireId,
        scope: ScopeId,
        segment: Option<Segment>,
        source: InstanceId,
        context: &Context,
    ) -> Option<WireId> {
        let decl = self.hir.wire(wire);
        let loc = decl.loc;
        if !self.count_item(loc, scope) {
            return None;
        }
        let id = WireId(index(self.wires.len()));
        let chain = self.chain(scope);
        let clause_scope = self.scope(ScopeKind::WireClause { wire: id }, Some(scope), chain);
        let attrs = self.wire_attrs(kind, wire, clause_scope);
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
        Some(id)
    }

    pub(in crate::semantics::analysis) fn new_use(
        &mut self,
        pattern: PatternId,
        placement: PlacementId,
        caller: ScopeId,
        naming: (Option<String>, Option<Segment>),
        context: &Context,
    ) -> Option<UseId> {
        let (name, segment) = naming;
        let owner = context.owner?;
        let loc = self.hir.placement(placement).loc;
        if !self.count_step(loc, caller) {
            return None;
        }
        let locals = self.use_locals(pattern, placement, caller);
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
        Some(id)
    }

    pub(in crate::semantics::analysis) fn use_locals(
        &mut self,
        pattern: PatternId,
        placement: PlacementId,
        caller: ScopeId,
    ) -> ScopeId {
        let decl = self.hir.pattern(pattern);
        let site = self.hir.placement(placement);
        let mut sources: Vec<Option<ParamSource>> = vec![None; decl.params.len()];
        if let Some((_, loc)) = site.flags.first() {
            self.error(
                Code::UseForm,
                *loc,
                caller,
                "a pattern use carries no flags",
            );
        }
        if site.body.is_some() {
            self.error(
                Code::UseForm,
                site.loc,
                caller,
                "a pattern use carries no body",
            );
        }
        if let Some(default) = site.default {
            match decl.params.iter().position(|param| param.is_default) {
                Some(position) => {
                    if let Some(slot) = sources.get_mut(position) {
                        *slot = Some(ParamSource::Arg {
                            expr: default,
                            scope: caller,
                        });
                    }
                }
                None => {
                    let loc = self.hir.expr(default).loc;
                    self.error(
                        Code::NoDefaultParameter,
                        loc,
                        caller,
                        "`/expr` supplies the parameter marked `default`, and this pattern has none",
                    );
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
                self.error(
                    Code::UnknownParameter,
                    attr.name.loc,
                    caller,
                    format!("the pattern has no parameter `{}`", attr.name.text),
                );
                continue;
            };
            let source = match attr.value {
                AttrValue::Expr(expr) => ParamSource::Arg {
                    expr,
                    scope: caller,
                },
                AttrValue::Set(value) => {
                    let node = self.push_node(Op::Bool(value), attr.loc, caller);
                    ParamSource::Bound(BoundVal::Scalar(Bound::Value(node)))
                }
            };
            match sources.get_mut(position) {
                Some(slot @ None) => *slot = Some(source),
                Some(Some(_)) => self.error(
                    Code::DuplicateArgument,
                    attr.name.loc,
                    caller,
                    format!("the parameter `{}` is supplied twice", attr.name.text),
                ),
                None => {}
            }
        }
        let mut params = Vec::new();
        for (param, source) in decl.params.iter().zip(sources) {
            params.push(match source {
                Some(source) => source,
                None if param.default.is_some() => ParamSource::Default,
                None => {
                    let name = param.name.as_ref().map_or("", |name| name.text.as_str());
                    self.error(
                        Code::MissingArgument,
                        site.loc,
                        caller,
                        format!("the parameter `{name}` has no default and no argument"),
                    );
                    ParamSource::Missing
                }
            });
        }
        let names = match self.memo.pattern_names.get(&pattern) {
            Some(names) => names.clone(),
            None => {
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
        let global = if self.hir.is_prelude_pattern(pattern) {
            self.prelude_global
        } else {
            self.source_global
        };
        let chain = self.chain(caller).push(site.loc);
        self.scope(
            ScopeKind::Locals(Locals {
                owner: LocalsOwner::Pattern(pattern),
                params,
                names,
                recursive: false,
            }),
            Some(global),
            chain,
        )
    }

    pub(in crate::semantics::analysis) fn stand_in(&self, pattern: PatternId) -> Option<StandIn> {
        self.collected
            .stand_ins
            .get(pattern.index())
            .copied()
            .flatten()
    }

    pub(in crate::semantics::analysis) fn stand_in_target(
        &self,
        mut pattern: PatternId,
    ) -> Option<PlacementId> {
        loop {
            let placement = match self.stand_in(pattern)? {
                StandIn::Node(placement) => return Some(placement),
                StandIn::Use(placement) => placement,
            };
            let kind = self.hir.placement(placement).kind.as_ref()?;
            let global = if self.hir.is_prelude_pattern(pattern) {
                self.prelude_global
            } else {
                self.source_global
            };
            match self.kind_decl(global, &kind.text)? {
                Decl::Pattern(next) if !self.is_cyclic(next) => pattern = next,
                Decl::Node(_) => return Some(placement),
                _ => return None,
            }
        }
    }

    pub(in crate::semantics::analysis) fn is_cyclic(&self, pattern: PatternId) -> bool {
        self.collected
            .cyclic_patterns
            .get(pattern.index())
            .copied()
            .unwrap_or(true)
    }

    fn wire_attrs(&mut self, kind: WireKind, wire: HirWireId, scope: ScopeId) -> Vec<AttrSource> {
        let hir = self.hir;
        let decl = hir.wire(wire);
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
    ) -> Vec<AttrSource> {
        let hir = self.hir;
        let decl = hir.placement(placement);
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
    ) -> Vec<AttrSource> {
        let vocabulary = self.vocabulary;
        let (kind_name, decls, default_attr) = match kind {
            ItemKind::Node(kind) => match vocabulary.node_kind(kind) {
                Some(decl) => (decl.name.as_str(), decl.attrs.as_slice(), decl.default_attr),
                None => return Vec::new(),
            },
            ItemKind::Wire(kind) => match vocabulary.wire_kind(kind) {
                Some(decl) => (decl.name.as_str(), decl.attrs.as_slice(), decl.default_attr),
                None => return Vec::new(),
            },
        };
        let mut sources: Vec<AttrSource> = Vec::new();
        let mut set = |engine: &mut Self, source: AttrSource| {
            if let Some(earlier) = sources.iter().find(|existing| existing.attr == source.attr) {
                let name = decls
                    .get(source.attr.index())
                    .map_or("", |decl| decl.name.as_str());
                let earlier = earlier.loc;
                engine.error_with_chain(
                    Code::DuplicateAttribute,
                    source.loc,
                    vec![earlier],
                    scope,
                    format!("the attribute `{name}` is set twice"),
                );
                return;
            }
            sources.push(source);
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
                ),
                None => self.error(
                    Code::UnknownFlag,
                    *flag_loc,
                    scope,
                    format!("`{kind_name}` has no attribute flagged `{flag}`"),
                ),
            }
        }
        if let Some(default) = default {
            let default_loc = self.hir.expr(default).loc;
            match default_attr {
                Some(attr) => set(
                    self,
                    AttrSource {
                        attr,
                        value: AttrValueSource::Expr(default, scope),
                        loc: default_loc,
                    },
                ),
                None => self.error(
                    Code::NoDefaultAttribute,
                    default_loc,
                    scope,
                    format!("`{kind_name}` has no default attribute for `/expr`"),
                ),
            }
        }
        for attr in attrs {
            let Some(position) = decls.iter().position(|decl| decl.name == attr.name.text) else {
                self.error(
                    Code::UnknownAttribute,
                    attr.name.loc,
                    scope,
                    format!("`{kind_name}` has no attribute `{}`", attr.name.text),
                );
                continue;
            };
            let value = match attr.value {
                AttrValue::Expr(expr) => AttrValueSource::Expr(expr, scope),
                AttrValue::Set(value) => {
                    let is_bool = decls
                        .get(position)
                        .is_some_and(|decl| decl.ty == symphonyscript_core::Ty::BOOL);
                    if !is_bool {
                        self.error(
                            Code::TypeMismatch,
                            attr.loc,
                            scope,
                            format!(
                                "`{}` alone or with `!` sets a `bool` attribute, and `{}` is not one",
                                attr.name.text, attr.name.text
                            ),
                        );
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
            );
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
                );
            }
        }
        sources
    }

    pub(in crate::semantics::analysis) fn item_loc(&self, item: ItemRef) -> Loc {
        match item {
            ItemRef::Instance(id) => self.instance(id).loc,
            ItemRef::Wire(id) => self.wire(id).loc,
            ItemRef::Use(id) => self.pattern_use(id).loc,
        }
    }

    pub(in crate::semantics::analysis) fn item_scope(&self, item: ItemRef) -> ScopeId {
        match item {
            ItemRef::Instance(id) => self.instance(id).scope,
            ItemRef::Wire(id) => self.wire(id).scope,
            ItemRef::Use(id) => self.pattern_use(id).caller,
        }
    }

    pub(in crate::semantics::analysis) fn item_description(&self, item: ItemRef) -> String {
        match item {
            ItemRef::Instance(id) => {
                let instance = self.instance(id);
                let kind = self
                    .vocabulary
                    .node_kind(instance.kind)
                    .map_or("", |decl| decl.name.as_str());
                match &instance.name {
                    Some(name) => format!("the `{kind}` `{name}`"),
                    None => format!("the `{kind}`"),
                }
            }
            ItemRef::Wire(id) => {
                let wire = self.wire(id);
                let kind = self
                    .vocabulary
                    .wire_kind(wire.kind)
                    .map_or("", |decl| decl.name.as_str());
                format!("the `{kind}` wire")
            }
            ItemRef::Use(id) => {
                let pattern = self.hir.pattern(self.pattern_use(id).pattern);
                let name = pattern.name.as_ref().map_or("", |name| name.text.as_str());
                format!("the use of `{name}`")
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum ItemKind {
    Node(NodeKind),
    Wire(WireKind),
}
