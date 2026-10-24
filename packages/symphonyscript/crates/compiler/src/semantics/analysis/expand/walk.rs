use std::collections::BTreeMap;
use std::rc::Rc;

use symphonyscript_core::Segment;

use super::iteration::Iteration;
use super::records::Context;
use crate::diagnostics::Code;
use crate::semantics::analysis::chain::Chain;
use crate::semantics::analysis::condition::Condition;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::entry::Entry;
use crate::semantics::analysis::ids::{ConditionsId, InstanceId, ScopeId, UseId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::path_parent::PathParent;
use crate::semantics::analysis::scope::ScopeKind;
use crate::semantics::analysis::site::Site;
use crate::semantics::analysis::val::Val;
use crate::semantics::collection::Decl;
use crate::semantics::ir::{BlockId, BodyItem, Loc, PatternId, PlacementId, Top};

#[derive(Clone)]
struct Walk {
    owner: InstanceId,
    path_parent: PathParent,
    conditions: Option<ConditionsId>,
    uses: Chain,
    bundle: Option<usize>,
}

enum Frame {
    Block {
        block: BlockId,
        next: usize,
        scope: ScopeId,
        walk: Walk,
    },
    Members {
        members: Vec<PlacementId>,
        next: usize,
        scope: ScopeId,
        walk: Walk,
    },
    For {
        var: Option<String>,
        iteration: Iteration,
        body: ForBody,
        scope: ScopeId,
        walk: Walk,
    },
    EndBundle {
        collector: usize,
        owner: InstanceId,
        loc: Loc,
        scope: ScopeId,
    },
}

#[derive(Clone)]
enum ForBody {
    Block(BlockId),
    Members(Vec<PlacementId>),
}

impl Engine<'_> {
    pub(crate) fn expand(&mut self) {
        let hir = self.hir;
        for (position, top) in hir.top.iter().enumerate() {
            if hir.is_prelude_top(position) {
                continue;
            }
            if let Top::Placement(placement) = top {
                self.top_level_item(*placement);
            }
        }
        while let Some(instance) = self.body_queue.pop_front() {
            self.expand_body(instance);
        }
    }

    fn top_level_item(&mut self, placement: PlacementId) {
        let site = self.hir.placement(placement);
        let (Some(kind), Some(name)) = (&site.kind, &site.name) else {
            return;
        };
        let global = self.source_global;
        let context = Context {
            owner: None,
            path_parent: None,
            conditions: None,
            uses: Chain::default(),
        };
        let segment = Some(Segment::Named(name.text.clone()));
        let instance = match self.kind_decl(global, &kind.text) {
            Some(Decl::Node(kind)) => {
                let created = self.new_instance(
                    kind,
                    placement,
                    global,
                    Some(name.text.clone()),
                    segment,
                    &context,
                );
                if site.body.is_some()
                    && let Some(created) = created
                {
                    self.body_queue.push_back(created);
                }
                created
            }
            Some(Decl::Pattern(pattern)) => {
                if self.is_cyclic(pattern) {
                    self.unknown_items(None, site.body);
                    return;
                }
                if self.stand_in(pattern).is_none() {
                    self.error(
                        Code::TopLevelPattern,
                        kind.loc,
                        global,
                        "only a pattern that stands in for a single node placement is used at top level",
                    );
                    self.unknown_items(None, site.body);
                    return;
                }
                self.stand_in_instance(
                    placement,
                    pattern,
                    global,
                    Some(name.text.clone()),
                    segment,
                    &context,
                )
            }
            Some(
                Decl::Input(_)
                | Decl::Let(_)
                | Decl::Fn(_)
                | Decl::Instance(_)
                | Decl::Builtin(_)
                | Decl::Wire(_)
                | Decl::Enum(_),
            )
            | None => None,
        };
        match instance {
            Some(instance) => {
                self.top_instances.insert(placement, instance);
                self.top_level.push(instance);
            }
            None => self.unknown_items(None, site.body),
        }
    }

    pub(in crate::semantics::analysis) fn stand_in_instance(
        &mut self,
        mut placement: PlacementId,
        mut pattern: PatternId,
        mut caller: ScopeId,
        name: Option<String>,
        segment: Option<Segment>,
        context: &Context,
    ) -> Option<InstanceId> {
        let mut conditions = context.conditions;
        let mut uses: Chain = context.uses.clone();
        loop {
            let site = self.hir.placement(placement);
            if let Some(gate) = site.gate {
                self.register_gate(gate, caller);
                conditions = self.push_condition(
                    conditions,
                    Condition {
                        expr: gate,
                        scope: caller,
                        holds: true,
                    },
                );
            }
            if !self.count_step(site.loc, caller) {
                return None;
            }
            uses = uses.push(site.loc);
            let locals = self.use_locals(pattern, placement, caller);
            let chain = self.chain(locals);
            let body = self.scope(
                ScopeKind::Body {
                    names: BTreeMap::new(),
                },
                Some(locals),
                chain,
            );
            let single = self.stand_in(pattern)?.placement();
            let kind = self.hir.placement(single).kind.clone()?;
            match self.kind_decl(body, &kind.text) {
                Some(Decl::Node(kind)) => {
                    let inner = Context {
                        owner: context.owner,
                        path_parent: context.path_parent,
                        conditions,
                        uses,
                    };
                    let id = self.new_instance(kind, single, body, name, segment, &inner)?;
                    if self.hir.placement(single).body.is_some() {
                        self.body_queue.push_back(id);
                    }
                    return Some(id);
                }
                Some(Decl::Pattern(next)) if !self.is_cyclic(next) => {
                    placement = single;
                    pattern = next;
                    caller = body;
                }
                Some(
                    Decl::Input(_)
                    | Decl::Let(_)
                    | Decl::Fn(_)
                    | Decl::Pattern(_)
                    | Decl::Instance(_)
                    | Decl::Builtin(_)
                    | Decl::Wire(_)
                    | Decl::Enum(_),
                )
                | None => return None,
            }
        }
    }

    fn expand_body(&mut self, instance: InstanceId) {
        let placement = self.instance(instance).placement;
        let Some(block) = self.hir.placement(placement).body else {
            return;
        };
        let scope = self.ensure_body(instance);
        let walk = Walk {
            owner: instance,
            path_parent: PathParent::Instance(instance),
            conditions: None,
            uses: self.instance(instance).uses.clone(),
            bundle: None,
        };
        self.walk(block, scope, walk);
        if let Some(record) = self.instances.get_mut(instance.index()) {
            record.expanded = true;
        }
    }

    pub(in crate::semantics::analysis) fn ensure_body(&mut self, instance: InstanceId) -> ScopeId {
        if let Some(scope) = self.instance(instance).body_scope {
            return scope;
        }
        let (parent, placement) = {
            let record = self.instance(instance);
            (record.scope, record.placement)
        };
        let chain = self.chain(parent);
        let scope = self.scope(
            ScopeKind::Body {
                names: BTreeMap::new(),
            },
            Some(parent),
            chain,
        );
        if let Some(record) = self.instances.get_mut(instance.index()) {
            record.body_scope = Some(scope);
        }
        if let Some(block) = self.hir.placement(placement).body {
            let sites = self.named_sites(block);
            if !sites.is_empty() {
                let uses = self.instance(instance).uses.clone();
                self.prebuild(
                    &sites,
                    scope,
                    instance,
                    PathParent::Instance(instance),
                    uses,
                );
            }
        }
        scope
    }

    pub(in crate::semantics::analysis) fn ensure_use_body(&mut self, id: UseId) -> ScopeId {
        if let Some(scope) = self.pattern_use(id).body {
            return scope;
        }
        let (locals, pattern, owner, uses, loc) = {
            let record = self.pattern_use(id);
            (
                record.locals,
                record.pattern,
                record.owner,
                record.uses.clone(),
                record.loc,
            )
        };
        let chain = self.chain(locals);
        let scope = self.scope(
            ScopeKind::Body {
                names: BTreeMap::new(),
            },
            Some(locals),
            chain,
        );
        if let Some(record) = self.uses.get_mut(id.index()) {
            record.body = Some(scope);
        }
        let sites = self.named_sites(self.hir.pattern(pattern).body);
        if !sites.is_empty() {
            self.prebuild(&sites, scope, owner, PathParent::Use(id), uses.push(loc));
        }
        scope
    }

    pub(in crate::semantics::analysis) fn child_named(
        &mut self,
        item: ItemRef,
        name: &str,
    ) -> Option<ItemRef> {
        let scope = match item {
            ItemRef::Instance(instance) => self.ensure_body(instance),
            ItemRef::Use(id) => self.ensure_use_body(id),
            ItemRef::Wire(_) => return None,
        };
        match &self.scope_of(scope).kind {
            ScopeKind::Body { names } => names.get(name).copied(),
            ScopeKind::Global { .. }
            | ScopeKind::Locals(_)
            | ScopeKind::ParamPrefix { .. }
            | ScopeKind::ForVar { .. }
            | ScopeKind::WireClause { .. } => None,
        }
    }

    fn named_sites(&mut self, root: BlockId) -> Rc<[(Site, Loc)]> {
        if let Some(sites) = self.memo.named_sites.get(&root) {
            return sites.clone();
        }
        let hir = self.hir;
        let mut blocks = vec![root];
        let mut sites: Vec<(Site, Loc)> = Vec::new();
        let named = |id: &PlacementId| hir.placement(*id).name.is_some();
        while let Some(block) = blocks.pop() {
            for item in &hir.block(block).items {
                match item {
                    BodyItem::Placement(id) if named(id) => {
                        sites.push((Site::Placement(*id), hir.placement(*id).loc))
                    }
                    BodyItem::Wire(id) if hir.wire(*id).name.is_some() => {
                        sites.push((Site::Wire(*id), hir.wire(*id).loc))
                    }
                    BodyItem::Bundle { members, .. } => {
                        sites.extend(
                            members
                                .iter()
                                .filter(|id| named(id))
                                .map(|id| (Site::Placement(*id), hir.placement(*id).loc)),
                        );
                    }
                    BodyItem::When {
                        arms, otherwise, ..
                    } => {
                        blocks.extend(arms.iter().map(|arm| arm.body));
                        blocks.extend(otherwise.iter().copied());
                    }
                    BodyItem::Placement(_) | BodyItem::Wire(_) | BodyItem::For { .. } => {}
                }
            }
        }
        sites.sort_by_key(|(_, loc)| *loc);
        let sites: Rc<[(Site, Loc)]> = sites.into();
        self.memo.named_sites.insert(root, sites.clone());
        sites
    }

    fn prebuild(
        &mut self,
        sites: &[(Site, Loc)],
        scope: ScopeId,
        owner: InstanceId,
        path_parent: PathParent,
        uses: Chain,
    ) {
        let context = Context {
            owner: Some(owner),
            path_parent: Some(path_parent),
            conditions: None,
            uses,
        };
        let owner_kind = self.instance(owner).kind;
        let mut names = BTreeMap::new();
        for (site, _) in sites.iter().copied() {
            let named = match site {
                Site::Placement(id) => self.hir.placement(id).name.clone(),
                Site::Wire(id) => self.hir.wire(id).name.clone(),
            };
            let Some(name) = named else {
                continue;
            };
            if names.contains_key(&name.text) {
                continue;
            }
            let attribute_clash = self
                .vocabulary
                .node_kind(owner_kind)
                .is_some_and(|decl| decl.attrs.iter().any(|attr| attr.name == name.text));
            if attribute_clash {
                let kind = self
                    .vocabulary
                    .node_kind(owner_kind)
                    .map_or("", |decl| decl.name.as_str());
                self.error(
                    Code::ChildNameClash,
                    name.loc,
                    scope,
                    format!(
                        "`{}` is an attribute of the parent's kind `{kind}`",
                        name.text
                    ),
                );
                continue;
            }
            let segment = Some(Segment::Named(name.text.clone()));
            let item = match site {
                Site::Placement(id) => {
                    let Some(kind) = self.hir.placement(id).kind.clone() else {
                        continue;
                    };
                    match self.kind_decl(scope, &kind.text) {
                        Some(Decl::Node(kind)) => {
                            let created = self.new_instance(
                                kind,
                                id,
                                scope,
                                Some(name.text.clone()),
                                segment,
                                &context,
                            );
                            if self.hir.placement(id).body.is_some()
                                && let Some(created) = created
                            {
                                self.body_queue.push_back(created);
                            }
                            created.map(ItemRef::Instance)
                        }
                        Some(Decl::Pattern(pattern)) if !self.is_cyclic(pattern) => {
                            if self.stand_in(pattern).is_some() {
                                self.stand_in_instance(
                                    id,
                                    pattern,
                                    scope,
                                    Some(name.text.clone()),
                                    segment,
                                    &context,
                                )
                                .map(ItemRef::Instance)
                            } else {
                                self.new_use(
                                    pattern,
                                    id,
                                    scope,
                                    (Some(name.text.clone()), segment),
                                    &context,
                                )
                                .map(ItemRef::Use)
                            }
                        }
                        Some(
                            Decl::Input(_)
                            | Decl::Let(_)
                            | Decl::Fn(_)
                            | Decl::Pattern(_)
                            | Decl::Instance(_)
                            | Decl::Builtin(_)
                            | Decl::Wire(_)
                            | Decl::Enum(_),
                        )
                        | None => None,
                    }
                }
                Site::Wire(id) => {
                    let kind = self.hir.wire(id).kind.text.clone();
                    match self.kind_decl(scope, &kind) {
                        Some(Decl::Wire(kind)) => self
                            .new_wire(kind, id, scope, segment, owner, &context)
                            .map(ItemRef::Wire),
                        Some(
                            Decl::Input(_)
                            | Decl::Let(_)
                            | Decl::Fn(_)
                            | Decl::Pattern(_)
                            | Decl::Instance(_)
                            | Decl::Builtin(_)
                            | Decl::Node(_)
                            | Decl::Enum(_),
                        )
                        | None => None,
                    }
                }
            };
            if let Some(item) = item {
                self.prebuilt.insert((site, scope), item);
                names.insert(name.text, item);
            }
        }
        if let Some(record) = self.scopes.get_mut(scope.index())
            && let ScopeKind::Body { names: slot } = &mut record.kind
        {
            *slot = names;
        }
    }

    fn walk(&mut self, block: BlockId, scope: ScopeId, walk: Walk) {
        let mut collectors: Vec<Vec<InstanceId>> = Vec::new();
        let mut frames = vec![Frame::Block {
            block,
            next: 0,
            scope,
            walk,
        }];
        while let Some(frame) = frames.pop() {
            if self.limit_reached {
                return;
            }
            match frame {
                Frame::Block {
                    block,
                    next,
                    scope,
                    walk,
                } => {
                    let Some(item) = self.hir.block(block).items.get(next).cloned() else {
                        continue;
                    };
                    frames.push(Frame::Block {
                        block,
                        next: next + 1,
                        scope,
                        walk: walk.clone(),
                    });
                    self.item(item, scope, &walk, &mut frames, &mut collectors);
                }
                Frame::Members {
                    members,
                    next,
                    scope,
                    walk,
                } => {
                    let Some(member) = members.get(next).copied() else {
                        continue;
                    };
                    frames.push(Frame::Members {
                        members,
                        next: next + 1,
                        scope,
                        walk: walk.clone(),
                    });
                    self.placement_item(member, scope, &walk, &mut frames, &mut collectors);
                }
                Frame::For {
                    var,
                    iteration,
                    body,
                    scope,
                    walk,
                } => {
                    let (value, iteration) = match iteration {
                        Iteration::Range { next, end, loc } => {
                            if next >= end {
                                continue;
                            }
                            let node = self.push_node(
                                crate::semantics::analysis::node::Op::Number(
                                    crate::semantics::ir::Number::Int(next),
                                    crate::semantics::ir::Suffix::None,
                                ),
                                loc,
                                scope,
                            );
                            (
                                Val::Scalar(node),
                                Iteration::Range {
                                    next: next + 1,
                                    end,
                                    loc,
                                },
                            )
                        }
                        Iteration::Values { values, next, loc } => {
                            let Some(value) = values.get(next).cloned() else {
                                continue;
                            };
                            (
                                value,
                                Iteration::Values {
                                    values,
                                    next: next + 1,
                                    loc,
                                },
                            )
                        }
                    };
                    if !self.count_step(iteration.loc(), scope) {
                        return;
                    }
                    let chain = self.chain(scope);
                    let var_scope = match &var {
                        Some(name) => self.scope(
                            ScopeKind::ForVar {
                                name: name.clone(),
                                value,
                            },
                            Some(scope),
                            chain,
                        ),
                        None => scope,
                    };
                    frames.push(Frame::For {
                        var,
                        iteration,
                        body: body.clone(),
                        scope,
                        walk: walk.clone(),
                    });
                    match body {
                        ForBody::Block(block) => frames.push(Frame::Block {
                            block,
                            next: 0,
                            scope: var_scope,
                            walk,
                        }),
                        ForBody::Members(members) => frames.push(Frame::Members {
                            members,
                            next: 0,
                            scope: var_scope,
                            walk,
                        }),
                    }
                }
                Frame::EndBundle {
                    collector,
                    owner,
                    loc,
                    scope,
                } => {
                    let members = collectors
                        .get_mut(collector)
                        .map(std::mem::take)
                        .unwrap_or_default();
                    let broken = self.broken_bundles.remove(&collector);
                    if members.is_empty() && !broken {
                        self.error(
                            Code::EmptyBundle,
                            loc,
                            scope,
                            "a bundle holds at least one member after expansion",
                        );
                    } else if let Some(record) = self.instances.get_mut(owner.index()) {
                        record.sequence.push(Entry::Bundle(members));
                    }
                }
            }
        }
    }

    fn item(
        &mut self,
        item: BodyItem,
        scope: ScopeId,
        walk: &Walk,
        frames: &mut Vec<Frame>,
        collectors: &mut Vec<Vec<InstanceId>>,
    ) {
        match item {
            BodyItem::Placement(id) => self.placement_item(id, scope, walk, frames, collectors),
            BodyItem::Wire(id) => {
                if let Some(collector) = walk.bundle {
                    let loc = self.hir.wire(id).loc;
                    self.error(
                        Code::BundleMember,
                        loc,
                        scope,
                        "a wire is not a member of a bundle",
                    );
                    self.broken_bundles.insert(collector);
                    return;
                }
                let kind = self.hir.wire(id).kind.text.clone();
                let Some(Decl::Wire(kind)) = self.kind_decl(scope, &kind) else {
                    return;
                };
                let wire = match self.prebuilt.get(&(Site::Wire(id), scope)) {
                    Some(ItemRef::Wire(existing)) => {
                        let existing = *existing;
                        if let Some(record) = self.wires.get_mut(existing.index()) {
                            record.conditions = walk.conditions;
                        }
                        Some(existing)
                    }
                    Some(ItemRef::Instance(_) | ItemRef::Use(_)) | None => {
                        let vocabulary = self.vocabulary;
                        let kind_name = vocabulary
                            .wire_kind(kind)
                            .map_or("", |decl| decl.name.as_str());
                        let segment = Some(self.segment(Some(walk.path_parent), None, kind_name));
                        let context = self.context(walk);
                        self.new_wire(kind, id, scope, segment, walk.owner, &context)
                    }
                };
                if let Some(wire) = wire
                    && let Some(record) = self.instances.get_mut(walk.owner.index())
                {
                    record.sequence.push(Entry::Wire(wire));
                }
            }
            BodyItem::Bundle { members, each, loc } => {
                if let Some(collector) = walk.bundle {
                    self.error(
                        Code::BundleMember,
                        loc,
                        scope,
                        "a bundle is not a member of a bundle",
                    );
                    self.broken_bundles.insert(collector);
                    return;
                }
                let collector = collectors.len();
                collectors.push(Vec::new());
                frames.push(Frame::EndBundle {
                    collector,
                    owner: walk.owner,
                    loc,
                    scope,
                });
                let mut inner = walk.clone();
                inner.bundle = Some(collector);
                if let Some(each) = each {
                    match self.iteration(each.iterable, scope) {
                        Some(iteration) => frames.push(Frame::For {
                            var: each.var.map(|var| var.text),
                            iteration,
                            body: ForBody::Members(each.members),
                            scope,
                            walk: inner.clone(),
                        }),
                        None => {
                            self.broken_bundles.insert(collector);
                            self.unknown_items(Some(walk.owner), None);
                        }
                    }
                }
                frames.push(Frame::Members {
                    members,
                    next: 0,
                    scope,
                    walk: inner,
                });
            }
            BodyItem::For {
                var,
                iterable,
                body,
                ..
            } => match self.iteration(iterable, scope) {
                Some(iteration) => frames.push(Frame::For {
                    var: var.map(|var| var.text),
                    iteration,
                    body: ForBody::Block(body),
                    scope,
                    walk: walk.clone(),
                }),
                None => {
                    if let Some(collector) = walk.bundle {
                        self.broken_bundles.insert(collector);
                    }
                    self.unknown_items(Some(walk.owner), Some(body));
                }
            },
            BodyItem::When {
                arms, otherwise, ..
            } => {
                let mut pending = Vec::new();
                let mut earlier = walk.conditions;
                for arm in &arms {
                    self.register_gate(arm.guard, scope);
                    let holds = self.push_condition(
                        earlier,
                        Condition {
                            expr: arm.guard,
                            scope,
                            holds: true,
                        },
                    );
                    earlier = self.push_condition(
                        earlier,
                        Condition {
                            expr: arm.guard,
                            scope,
                            holds: false,
                        },
                    );
                    pending.push((arm.body, holds));
                }
                if let Some(otherwise) = otherwise {
                    pending.push((otherwise, earlier));
                }
                for (block, conditions) in pending.into_iter().rev() {
                    let mut inner = walk.clone();
                    inner.conditions = conditions;
                    frames.push(Frame::Block {
                        block,
                        next: 0,
                        scope,
                        walk: inner,
                    });
                }
            }
        }
    }

    fn placement_item(
        &mut self,
        id: PlacementId,
        scope: ScopeId,
        walk: &Walk,
        frames: &mut Vec<Frame>,
        collectors: &mut [Vec<InstanceId>],
    ) {
        let hir = self.hir;
        let Some(kind) = hir.placement(id).kind.as_ref() else {
            return;
        };
        let has_body = hir.placement(id).body.is_some();
        let instance = match self.kind_decl(scope, &kind.text) {
            Some(Decl::Node(node_kind)) => {
                if let Some(existing) = self.reuse(id, scope, walk) {
                    Some(existing)
                } else {
                    let segment =
                        Some(self.segment(Some(walk.path_parent), None, kind.text.as_str()));
                    let context = self.context(walk);
                    let created = self.new_instance(node_kind, id, scope, None, segment, &context);
                    if has_body && let Some(created) = created {
                        self.body_queue.push_back(created);
                    }
                    created
                }
            }
            Some(Decl::Pattern(pattern)) => {
                if self.is_cyclic(pattern) {
                    if let Some(collector) = walk.bundle {
                        self.broken_bundles.insert(collector);
                    }
                    self.unknown_items(Some(walk.owner), None);
                    return;
                }
                if self.stand_in(pattern).is_some() {
                    if let Some(collector) = walk.bundle
                        && self
                            .stand_in_target(pattern)
                            .is_some_and(|target| self.hir.placement(target).body.is_some())
                    {
                        let loc = self.hir.placement(id).loc;
                        self.error(
                            Code::BundleMember,
                            loc,
                            scope,
                            "a member of a bundle has no body",
                        );
                        self.broken_bundles.insert(collector);
                        return;
                    }
                    let existing = self.reuse(id, scope, walk);
                    match existing {
                        Some(existing) => Some(existing),
                        None => {
                            let segment = Some(self.segment(
                                Some(walk.path_parent),
                                None,
                                kind.text.as_str(),
                            ));
                            let context = self.context(walk);
                            self.stand_in_instance(id, pattern, scope, None, segment, &context)
                        }
                    }
                } else {
                    let use_id = match self.prebuilt.get(&(Site::Placement(id), scope)) {
                        Some(ItemRef::Use(existing)) => {
                            let existing = *existing;
                            if let Some(record) = self.uses.get_mut(existing.index()) {
                                record.conditions = walk.conditions;
                            }
                            existing
                        }
                        Some(ItemRef::Instance(_) | ItemRef::Wire(_)) | None => {
                            let segment = Some(self.segment(
                                Some(walk.path_parent),
                                None,
                                kind.text.as_str(),
                            ));
                            let context = self.context(walk);
                            match self.new_use(pattern, id, scope, (None, segment), &context) {
                                Some(created) => created,
                                None => return,
                            }
                        }
                    };
                    let body_scope = self.ensure_use_body(use_id);
                    let (gate, loc, uses) = {
                        let record = self.pattern_use(use_id);
                        (self.hir.placement(id).gate, record.loc, record.uses.clone())
                    };
                    let mut conditions = walk.conditions;
                    if let Some(gate) = gate {
                        self.register_gate(gate, scope);
                        conditions = self.push_condition(
                            conditions,
                            Condition {
                                expr: gate,
                                scope,
                                holds: true,
                            },
                        );
                    }
                    let inner = Walk {
                        owner: walk.owner,
                        path_parent: PathParent::Use(use_id),
                        conditions,
                        uses: uses.push(loc),
                        bundle: walk.bundle,
                    };
                    frames.push(Frame::Block {
                        block: self.hir.pattern(pattern).body,
                        next: 0,
                        scope: body_scope,
                        walk: inner,
                    });
                    return;
                }
            }
            Some(
                Decl::Input(_)
                | Decl::Let(_)
                | Decl::Fn(_)
                | Decl::Instance(_)
                | Decl::Builtin(_)
                | Decl::Wire(_)
                | Decl::Enum(_),
            )
            | None => None,
        };
        let Some(instance) = instance else {
            if let Some(collector) = walk.bundle {
                self.broken_bundles.insert(collector);
            }
            self.unknown_items(Some(walk.owner), self.hir.placement(id).body);
            return;
        };
        match walk.bundle {
            Some(collector) => {
                if has_body {
                    let loc = self.hir.placement(id).loc;
                    self.error(
                        Code::BundleMember,
                        loc,
                        scope,
                        "a member of a bundle has no body",
                    );
                    self.broken_bundles.insert(collector);
                    return;
                }
                if let Some(members) = collectors.get_mut(collector) {
                    members.push(instance);
                }
            }
            None => {
                if let Some(record) = self.instances.get_mut(walk.owner.index()) {
                    record.sequence.push(Entry::Child(instance));
                }
            }
        }
    }

    fn reuse(&mut self, id: PlacementId, scope: ScopeId, walk: &Walk) -> Option<InstanceId> {
        let Some(ItemRef::Instance(existing)) =
            self.prebuilt.get(&(Site::Placement(id), scope)).copied()
        else {
            return None;
        };
        let own = self.instance(existing).conditions;
        let conditions = self.joined_conditions(walk.conditions, own);
        if let Some(record) = self.instances.get_mut(existing.index()) {
            record.conditions = conditions;
        }
        Some(existing)
    }

    fn context(&self, walk: &Walk) -> Context {
        Context {
            owner: Some(walk.owner),
            path_parent: Some(walk.path_parent),
            conditions: walk.conditions,
            uses: walk.uses.clone(),
        }
    }
}
