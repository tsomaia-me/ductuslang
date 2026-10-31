use symphonyscript_core::{AttrDecl, AttrId, NodeKind, WireKind};

use super::found::Found;
use super::members::Member;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::attr_source::AttrSource;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_slice::BoundSlice;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastSet;
use crate::semantics::analysis::frame_read::FrameRead;
use crate::semantics::analysis::ids::{NodeId, ScopeId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::rec_family::RecFamily;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::shape::Shape;
use crate::semantics::analysis::site::Site;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::collection::Decl;
use crate::semantics::ir::{
    ExprId, ExprKind, FnId, Ident, Loc, PatternId, PlacementId, WireId as HirWireId,
};

enum Shortcut<T> {
    Pending,
    Declined,
    Read(T),
}

enum Written {
    Node(NodeKind, PlacementId, ScopeId),
    Wire(WireKind),
    Use(PatternId, ScopeId),
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn field(
        &mut self,
        base: ExprId,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        match self.member(base, name, loc, scope, need)? {
            Member::Pending => return Ok(None),
            Member::Read(val) => return Ok(Some(val)),
            Member::Declined => {}
        }
        if let Some(mut segments) = self.to_path(base)? {
            segments.push(name.clone());
            return self.destination_read(&segments, loc, scope, need);
        }
        match &self.hir.expr(base)?.kind {
            ExprKind::To => return Ok(Some(Val::Error)),
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Name(_)
            | ExprKind::Variant { .. }
            | ExprKind::Convert(..)
            | ExprKind::Array(_)
            | ExprKind::Field(..)
            | ExprKind::Index(..)
            | ExprKind::IndexFromEnd(..)
            | ExprKind::Slice { .. }
            | ExprKind::History { .. }
            | ExprKind::Stepped(_)
            | ExprKind::Call(..)
            | ExprKind::Unary(..)
            | ExprKind::Binary(..)
            | ExprKind::If(..)
            | ExprKind::Match { .. }
            | ExprKind::For(_) => {}
        }
        let base_val = some!(self.val(Query::Expr(base, scope), need));
        let node = match base_val {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(
                    Code::UnknownAttribute,
                    loc,
                    scope,
                    "an array has no attributes; index it first",
                )?;
                return Ok(Some(Val::Error));
            }
            Val::Error => {
                self.written_field(base, name, loc, scope)?;
                return Ok(Some(Val::Error));
            }
        };
        let item = match &self.node(self.through_uses(node)?)?.op {
            Op::NodeRef(item) => *item,
            Op::Bare(_) => {
                let bare = self.through_uses(node)?;
                self.unplaced_variant(bare)?;
                return Ok(Some(Val::Error));
            }
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Pi
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
            | Op::Native { .. } => {
                let sig = some!(self.sig(node, need));
                if sig.is_error() {
                    return Ok(Some(Val::Error));
                }
                if sig.open && self.in_probe(scope)? {
                    return Ok(Some(Val::Scalar(self.open_source(node)?)));
                }
                if sig.open && sig.frame {
                    return self.frame_path(node, name, loc, scope, need);
                }
                if sig.kind != Kind::Node {
                    self.error(
                        Code::UnknownAttribute,
                        loc,
                        scope,
                        format!("`.{}` reads an attribute of an instance or a wire, and this is a value", name.text),
                    )?;
                    return Ok(Some(Val::Error));
                }
                if sig.reactive {
                    self.error(
                        Code::RuntimePath,
                        loc,
                        scope,
                        "a path starts at an instance chosen at compile time",
                    )?;
                    return Ok(Some(Val::Error));
                }
                if sig.frame {
                    return self.frame_path(node, name, loc, scope, need);
                }
                match some!(self.cval(Query::Eval(node, EvalTy::Node), need)) {
                    CVal::Node(item) => item,
                    CVal::Int(_)
                    | CVal::F32(_)
                    | CVal::F64(_)
                    | CVal::Bool(_)
                    | CVal::Enum(..)
                    | CVal::Str(_)
                    | CVal::Array(_)
                    | CVal::Error => return Ok(Some(Val::Error)),
                }
            }
        };
        if let Some(Query::Expr(expr, _)) = self.current
            && let Some(path) = self
                .open_candidates
                .get(&(expr, item, name.text.clone()))
                .cloned()
            && self.live_candidate_cycle(item, &name.text, &path)?
        {
            return Ok(Some(Val::Error));
        }
        self.step(item, name, loc, scope, need)
    }

    fn open_source(&self, start: NodeId) -> Result<NodeId, Internal> {
        let mut seen: FastSet<NodeId> = FastSet::default();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if !seen.insert(node) {
                continue;
            }
            let Some(sig) = self.memo.sigs.get(&node) else {
                continue;
            };
            if !sig.open {
                continue;
            }
            if sig.kind == Kind::Any {
                return Ok(node);
            }
            stack.extend(self.node(node)?.op.operands().into_iter().rev());
        }
        Err(Internal::missed("open value"))
    }

    fn frame_path(
        &mut self,
        root: NodeId,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let instances = some!(self.reference_instances(root, need)?);
        let mut reads: Vec<(ItemRef, Val)> = Vec::with_capacity(instances.len());
        let mut open = Vec::new();
        let mut missing = false;
        let requester = self.current;
        let path_expr = match requester {
            Some(Query::Expr(expr, _)) => Some(expr),
            Some(
                Query::Let(..)
                | Query::Param(..)
                | Query::Input(_)
                | Query::State(_)
                | Query::Attr(..)
                | Query::Destination(_)
                | Query::Sig(_)
                | Query::Eval(..),
            )
            | None => None,
        };
        for instance in &instances {
            crate::semantics::counted_walk::CountedWalk::PathsFramePath.step();
            if let Some(expr) = path_expr
                && let Some(path) = self
                    .open_candidates
                    .get(&(expr, *instance, name.text.clone()))
            {
                open.push((*instance, path.clone()));
                continue;
            }
            let before = need.len();
            match self.candidate_read(*instance, name, loc, scope, need)? {
                Some(Some(val)) => reads.push((*instance, val)),
                Some(None) => {}
                None => {
                    missing = true;
                    if let (Some(requester), Some(expr)) = (requester, path_expr) {
                        for dep in need.get(before..).unwrap_or_default() {
                            self.candidate_edges
                                .insert((requester, *dep), (expr, *instance, name.text.clone()));
                        }
                    }
                }
            }
        }
        if missing {
            return Ok(None);
        }
        let Some((_, last)) = reads.last() else {
            if instances.is_empty() && !self.known_sig(root)?.open {
                return Ok(Some(Val::Error));
            }
            let join = self.push_node(Op::Bool(false), loc, scope)?;
            self.memo.open_joins.insert(join);
            let head = self.push_node(Op::Use(join), loc, scope)?;
            self.frame_head(
                head,
                FrameRead {
                    start: root,
                    arms: Vec::new(),
                    open,
                    name: name.clone(),
                    loc,
                    position: 0,
                    scope,
                },
            )?;
            return Ok(Some(Val::Scalar(head)));
        };
        if reads.iter().any(|(_, val)| matches!(val, Val::Error)) {
            return Ok(Some(Val::Error));
        }
        let shape = last.shape();
        if reads
            .iter()
            .any(|(_, val)| val.shape().lengths() != shape.lengths())
        {
            self.error(Code::TypeMismatch, loc, scope, CHOSEN_PATHS)?;
            return Ok(Some(Val::Error));
        }
        let passed = self.push_node(Op::Bool(false), loc, scope)?;
        self.memo.frame_joins.insert(passed);
        let mut arms: Vec<(ItemRef, Vec<NodeId>)> = Vec::with_capacity(reads.len());
        for (item, val) in reads {
            arms.push((item, val.nodes()));
        }
        let mut elements = Vec::with_capacity(shape.count());
        for position in 0..shape.count() {
            let mut chosen = Vec::with_capacity(arms.len());
            for (item, nodes) in &arms {
                let read = nodes
                    .get(position)
                    .copied()
                    .ok_or(Internal::missed("path element"))?;
                chosen.push((*item, read));
            }
            let mut joined: Option<NodeId> = None;
            for (_, read) in chosen.iter().rev() {
                joined = Some(match joined {
                    None => *read,
                    Some(after) => self.push_node(Op::If(passed, *read, after), loc, scope)?,
                });
            }
            let joined = joined.ok_or(Internal::missed("path element"))?;
            let head = self.push_node(Op::Use(joined), loc, scope)?;
            self.frame_head(
                head,
                FrameRead {
                    start: root,
                    arms: chosen,
                    open: open.clone(),
                    name: name.clone(),
                    loc,
                    position,
                    scope,
                },
            )?;
            elements.push(head);
        }
        Ok(Some(self.shaped(shape, elements)))
    }

    fn frame_head(&mut self, head: NodeId, read: FrameRead) -> Result<(), Internal> {
        if let Some(template) = self.template_of(read.scope)? {
            self.memo
                .template_heads
                .entry(template)
                .or_default()
                .push(head);
        }
        self.memo.frame_joins.insert(head);
        self.memo.frame_reads.insert(head, read);
        Ok(())
    }

    pub(in crate::semantics::analysis) fn open_arm(
        &mut self,
        read: &FrameRead,
        item: ItemRef,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<NodeId>>, Internal> {
        let key = (read.start, read.position, item);
        if let Some(arm) = self.memo.open_arms.get(&key) {
            return Ok(Some(*arm));
        }
        let val = some!(self.step(item, &read.name, read.loc, read.scope, need)?);
        let arm = val.nodes().get(read.position).copied();
        if arm.is_none() {
            self.error(Code::TypeMismatch, read.loc, read.scope, CHOSEN_PATHS)?;
        }
        self.memo.open_arms.insert(key, arm);
        Ok(Some(arm))
    }

    fn candidate_read(
        &mut self,
        item: ItemRef,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<Val>>, Internal> {
        if self.child_named(item, &name.text)?.is_none()
            && self.attr_named(item, &name.text)?.is_none()
        {
            return Ok(Some(None));
        }
        Ok(self.step(item, name, loc, scope, need)?.map(Some))
    }

    pub(in crate::semantics::analysis) fn reference_instances(
        &self,
        root: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Vec<ItemRef>>, Internal> {
        let mut instances = Vec::new();
        let mut held: FastSet<ItemRef> = FastSet::default();
        let mut seen: FastSet<NodeId> = FastSet::default();
        let mut stack = vec![root];
        let mut pending = false;
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::PathsReferenceInstances.step();
            if !seen.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::NodeRef(item) => {
                    if held.insert(*item) {
                        instances.push(*item);
                    }
                }
                Op::Use(inner) | Op::Chosen(inner) => stack.push(*inner),
                Op::If(_, then, otherwise) => stack.extend([*otherwise, *then]),
                Op::Match {
                    arms, otherwise, ..
                } => {
                    stack.extend(*otherwise);
                    stack.extend(arms.iter().rev().map(|(_, value)| *value));
                }
                Op::Pick { items, .. } | Op::Array(items) => stack.extend(items.iter().rev()),
                Op::Element { base, .. } | Op::Slice { base, .. } => stack.push(*base),
                Op::Join(lhs, rhs) => stack.extend([*rhs, *lhs]),
                Op::For(for_node) => stack.extend([for_node.body, for_node.start]),
                Op::Slot(slot) => stack.extend(self.slot(*slot)?.source),
                Op::Arg(_) => {
                    for item in self
                        .memo
                        .arg_instances
                        .get(&node)
                        .into_iter()
                        .flat_map(|among| among.iter())
                    {
                        if held.insert(*item) {
                            instances.push(*item);
                        }
                    }
                }
                Op::Recursive { function, .. } => match self.family_roots(node, *function, need)? {
                    Some(roots) => stack.extend(roots.into_iter().rev()),
                    None => pending = true,
                },
                Op::Bool(_) if self.memo.open_joins.contains(&node) => {}
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Bare(_)
                | Op::Pi
                | Op::Read(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::Convert(..)
                | Op::Annotate(..)
                | Op::Builtin(..)
                | Op::ToRead { .. }
                | Op::History { .. }
                | Op::Stepped(_)
                | Op::Native { .. } => return Err(Internal::missed("path start")),
            }
        }
        Ok((!pending).then_some(instances))
    }

    fn family_roots(
        &self,
        call: NodeId,
        function: FnId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Vec<NodeId>>, Internal> {
        let Some(key) = self.memo.call_families.get(&call) else {
            return Ok(Some(Vec::new()));
        };
        let mut scopes = Vec::new();
        if let Some(RecFamily::Probing(scope) | RecFamily::Settling { scope, .. }) =
            self.memo.rec_families.get(key)
        {
            scopes.push(*scope);
        }
        scopes.extend(
            self.memo
                .family_scopes
                .get(key)
                .into_iter()
                .flat_map(|settled| settled.iter().rev())
                .copied(),
        );
        scopes.extend(self.memo.family_probes.get(key).copied());
        let decl = self.hir.fn_decl(function)?;
        let body = decl.value;
        let mut waiting = None;
        for scope in scopes {
            match self.memo.exprs.get(&(body, scope)) {
                Some(val) => return Ok(Some(val.nodes().into_iter().collect())),
                None => {
                    let query = Query::Expr(body, scope);
                    if waiting.is_none() && !self.in_progress(query) {
                        waiting = Some(query);
                    }
                }
            }
        }
        match waiting {
            Some(query) => {
                need.push(query);
                Ok(None)
            }
            None => Ok(Some(Vec::new())),
        }
    }

    fn step(
        &mut self,
        item: ItemRef,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        if let Some(child) = self.child_named(item, &name.text)? {
            return Ok(Some(self.scalar(Op::NodeRef(child), loc, scope)?));
        }
        Ok(match self.attr_named(item, &name.text)? {
            Some(attr) => {
                let bound = some!(self.bound(Query::Attr(item, attr), need));
                Some(self.read(&bound, loc, scope)?)
            }
            None if self.may_name(item, &name.text)? => Some(Val::Error),
            None => {
                let owner = self.item_description(item)?;
                self.error(
                    Code::UnknownAttribute,
                    name.loc,
                    scope,
                    format!("{owner} has no child, wire or attribute `{}`", name.text),
                )?;
                Some(Val::Error)
            }
        })
    }

    fn written_field(
        &mut self,
        base: ExprId,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<(), Internal> {
        if let ExprKind::Field(_, segment) = &self.hir.expr(base)?.kind
            && self.memo.site_values.contains(&(segment.loc, scope))
        {
            return self.error(
                Code::UnknownAttribute,
                loc,
                scope,
                format!(
                    "`.{}` reads an attribute of an instance or a wire, and this is a value",
                    name.text
                ),
            );
        }
        let Some((site, kinds)) = self.written_base(base, scope)? else {
            return Ok(());
        };
        let Some(written) = self.written(site, kinds)? else {
            return Ok(());
        };
        let (body, inner) = match written {
            Written::Node(_, placement, inner) => (self.hir.placement(placement)?.body, inner),
            Written::Use(pattern, inner) => (Some(self.hir.pattern(pattern)?.body), inner),
            Written::Wire(_) => (None, kinds),
        };
        if let Some(body) = body {
            for (child, _) in self.named_sites(body)?.iter() {
                if self
                    .site_name(*child)?
                    .is_some_and(|child| child.text == name.text)
                {
                    self.memo
                        .site_paths
                        .insert((name.loc, scope), (*child, inner));
                    return Ok(());
                }
            }
        }
        let attrs = match written {
            Written::Node(kind, ..) => self.node_decl(kind)?.attrs.as_slice(),
            Written::Wire(kind) => self.wire_decl(kind)?.attrs.as_slice(),
            Written::Use(..) => &[],
        };
        if attrs.iter().any(|attr| attr.name == name.text) {
            self.memo.site_values.insert((name.loc, scope));
            return Ok(());
        }
        if let Written::Node(_, placement, _) = written
            && self
                .hir
                .placement(placement)?
                .skipped_block
                .holds(&name.text)
        {
            return Ok(());
        }
        if let Some(body) = body
            && self.body_may_name(body, &name.text)?
        {
            return Ok(());
        }
        let owner = match written {
            Written::Node(kind, ..) => {
                let own = self.site_name(site)?.map_or("", |own| own.text.as_str());
                format!("the `{}` named `{own}`", self.node_decl(kind)?.name)
            }
            Written::Wire(kind) => format!("the `{}` wire", self.wire_decl(kind)?.name),
            Written::Use(pattern, _) => {
                let pattern = self.hir.pattern(pattern)?.name.as_ref();
                format!(
                    "the use of `{}`",
                    pattern.map_or("", |name| name.text.as_str())
                )
            }
        };
        self.error(
            Code::UnknownAttribute,
            name.loc,
            scope,
            format!("{owner} has no child, wire or attribute `{}`", name.text),
        )
    }

    fn written_base(
        &self,
        base: ExprId,
        scope: ScopeId,
    ) -> Result<Option<(Site, ScopeId)>, Internal> {
        let name = match &self.hir.expr(base)?.kind {
            ExprKind::Name(name) => name,
            ExprKind::Field(_, segment) => {
                return Ok(self.memo.site_paths.get(&(segment.loc, scope)).copied());
            }
            _ => return Ok(None),
        };
        let mut current = Some(scope);
        while let Some(id) = current {
            crate::semantics::counted_walk::CountedWalk::PathsWrittenBase.step();
            let record = self.scope_of(id)?;
            match &record.kind {
                ScopeKind::ForVar { name: var, .. } if var == name => {
                    return Ok(self.memo.site_scopes.get(&id).map(|site| (*site, id)));
                }
                ScopeKind::ForVar { .. } | ScopeKind::WrittenClause { .. } => {}
                ScopeKind::Body { names } if !names.contains_key(name) => {}
                ScopeKind::Body { .. }
                | ScopeKind::Global { .. }
                | ScopeKind::Locals(_)
                | ScopeKind::ParamPrefix { .. }
                | ScopeKind::WireClause { .. } => return Ok(None),
            }
            current = record.parent;
        }
        Ok(None)
    }

    fn written(&self, site: Site, kinds: ScopeId) -> Result<Option<Written>, Internal> {
        let kind = match site {
            Site::Placement(placement) => self.hir.placement(placement)?.kind.as_ref(),
            Site::Wire(wire) => Some(&self.hir.wire(wire)?.kind),
        };
        let Some(kind) = kind else {
            return Ok(None);
        };
        Ok(match (site, self.kind_decl(kinds, &kind.text)?) {
            (Site::Placement(placement), Some(Decl::Node(node))) => {
                Some(Written::Node(node, placement, kinds))
            }
            (Site::Placement(_), Some(Decl::Pattern(pattern))) => {
                match self.stand_in_target(pattern)? {
                    Some((target, node, inner)) => Some(Written::Node(node, target, inner)),
                    None if self.is_placeholder(pattern)? || self.is_cyclic(pattern)? => None,
                    None if self.hir.is_prelude_pattern(pattern) => {
                        Some(Written::Use(pattern, self.prelude_global))
                    }
                    None => Some(Written::Use(pattern, self.source_global)),
                }
            }
            (Site::Wire(_), Some(Decl::Wire(wire))) => Some(Written::Wire(wire)),
            _ => None,
        })
    }

    fn site_name(&self, site: Site) -> Result<Option<&Ident>, Internal> {
        Ok(match site {
            Site::Placement(placement) => self.hir.placement(placement)?.name.as_ref(),
            Site::Wire(wire) => self.hir.wire(wire)?.name.as_ref(),
        })
    }

    fn to_path(&self, base: ExprId) -> Result<Option<Vec<Ident>>, Internal> {
        let mut segments = Vec::new();
        let mut current = base;
        loop {
            match &self.hir.expr(current)?.kind {
                ExprKind::To => break,
                ExprKind::Field(inner, segment) => {
                    segments.push(segment.clone());
                    current = *inner;
                }
                _ => return Ok(None),
            }
        }
        segments.reverse();
        Ok(Some(segments))
    }

    fn destination_read(
        &mut self,
        path: &[Ident],
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let (wire, candidates) = match self.enclosing_wire(scope)? {
            Some(wire) => {
                let destination = some!(self.destination(Query::Destination(wire), need));
                (Some(wire), destination.candidates())
            }
            None => match self.written_clause(scope)? {
                Some((written, outer)) => {
                    (None, some!(self.written_destination(written, outer, need)?))
                }
                None => return Ok(Some(Val::Error)),
            },
        };
        if candidates
            .iter()
            .any(|candidate| !matches!(candidate, ItemRef::Instance(_)))
        {
            return Ok(Some(Val::Error));
        }
        let Some((last, steps)) = path.split_last() else {
            return Ok(Some(Val::Error));
        };
        let text: Vec<&str> = path.iter().map(|segment| segment.text.as_str()).collect();
        if wire.is_none() && candidates.len() > 1 {
            self.written_choice_read(&candidates, steps, last, &text.join("."), loc, scope)?;
            return Ok(Some(Val::Error));
        }
        let mut attrs = Vec::new();
        let missing = if candidates.len() > 1 {
            Code::ToCandidates
        } else {
            Code::UnknownAttribute
        };
        for candidate in candidates {
            let mut item = candidate;
            for step in steps {
                match self.child_named(item, &step.text)? {
                    Some(child) => item = child,
                    None if self.may_name(item, &step.text)? => return Ok(Some(Val::Error)),
                    None => {
                        let owner = self.item_description(item)?;
                        self.error(
                            missing,
                            step.loc,
                            scope,
                            format!(
                                "{owner} has no named child, wire or pattern use `{}`, so `to.{}` reads nothing there",
                                step.text,
                                text.join(".")
                            ),
                        )?;
                        return Ok(Some(Val::Error));
                    }
                }
            }
            if self.child_named(item, &last.text)?.is_some() {
                self.error(
                    Code::MisplacedTo,
                    last.loc,
                    scope,
                    format!(
                        "`to.{}` ends at an instance or a wire; a `to` path ends at an attribute",
                        text.join(".")
                    ),
                )?;
                return Ok(Some(Val::Error));
            }
            match self.attr_named(item, &last.text)? {
                Some(attr) => attrs.push((item, attr)),
                None if self.may_name(item, &last.text)? => return Ok(Some(Val::Error)),
                None => {
                    let owner = self.item_description(item)?;
                    let code = if steps.is_empty() {
                        Code::UnknownAttribute
                    } else {
                        missing
                    };
                    self.error(
                        code,
                        last.loc,
                        scope,
                        format!("the destination, {owner}, has no attribute `{}`", last.text),
                    )?;
                    return Ok(Some(Val::Error));
                }
            }
        }
        let Some(wire) = wire else {
            return Ok(Some(Val::Error));
        };
        if attrs.is_empty() {
            return Ok(Some(Val::Error));
        }
        if let [(item, attr)] = attrs.as_slice()
            && self.to_read_layout(&attrs)?.is_some()
        {
            let bound = some!(self.bound(Query::Attr(*item, *attr), need));
            return Ok(Some(self.read(&bound, loc, scope)?));
        }
        Ok(Some(self.scalar(
            Op::ToRead {
                wire,
                attr: text.join("."),
                attrs,
            },
            loc,
            scope,
        )?))
    }

    fn written_choice_read(
        &mut self,
        candidates: &[ItemRef],
        steps: &[Ident],
        last: &Ident,
        text: &str,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<(), Internal> {
        let (mut instances, mut lacking) = (0, Vec::new());
        'candidates: for candidate in candidates {
            let mut item = *candidate;
            for step in steps {
                match self.child_named(item, &step.text)? {
                    Some(child) => item = child,
                    None if self.may_name(item, &step.text)? => return Ok(()),
                    None => {
                        lacking.push(*candidate);
                        continue 'candidates;
                    }
                }
            }
            if self.child_named(item, &last.text)?.is_some() {
                instances += 1;
            } else if self.attr_named(item, &last.text)?.is_some()
                || self.may_name(item, &last.text)?
            {
                return Ok(());
            } else {
                lacking.push(*candidate);
            }
        }
        if instances == candidates.len() {
            return self.error(
                Code::MisplacedTo,
                last.loc,
                scope,
                format!(
                    "`to.{text}` ends at an instance or a wire; a `to` path ends at an attribute"
                ),
            );
        }
        let mut related = Vec::with_capacity(lacking.len());
        for candidate in &lacking {
            let owner = self.item_description(*candidate)?;
            related.push((
                self.item_loc(*candidate)?,
                format!("{owner} has no `to.{text}`"),
            ));
        }
        let summary = match (
            instances,
            candidates.first(),
            candidates.get(1),
            lacking.first(),
        ) {
            (0, Some(first), Some(second), _) => {
                let first = self.item_description(*first)?;
                let second = self.item_description(*second)?;
                format!(
                    "no candidate of the destination, {first} and {second} among them, has `to.{text}`"
                )
            }
            (_, _, _, Some(first)) => {
                let first = self.item_description(*first)?;
                format!(
                    "no candidate of the destination has the attribute `to.{text}`: {first} has no such path, and the others end it at an instance or a wire"
                )
            }
            _ => return Ok(()),
        };
        self.error_with_chain(Code::UnknownAttribute, loc, related, scope, summary)
    }

    fn written_clause(&self, scope: ScopeId) -> Result<Option<(HirWireId, ScopeId)>, Internal> {
        let mut current = Some(scope);
        while let Some(id) = current {
            crate::semantics::counted_walk::CountedWalk::PathsWrittenClause.step();
            let record = self.scope_of(id)?;
            match record.kind {
                ScopeKind::WrittenClause { wire } => {
                    return Ok(record.parent.map(|outer| (wire, outer)));
                }
                ScopeKind::WireClause { .. } | ScopeKind::Global { .. } => return Ok(None),
                ScopeKind::Locals(_)
                | ScopeKind::ParamPrefix { .. }
                | ScopeKind::Body { .. }
                | ScopeKind::ForVar { .. } => {}
            }
            current = record.parent;
        }
        Ok(None)
    }

    fn written_destination(
        &mut self,
        wire: HirWireId,
        outer: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Vec<ItemRef>>, Internal> {
        let mut exprs = vec![self.hir.wire(wire)?.destination];
        let mut roots = Vec::new();
        while let Some(expr) = exprs.pop() {
            crate::semantics::counted_walk::CountedWalk::PathsWrittenDestination.step();
            match &self.hir.expr(expr)?.kind {
                ExprKind::If(_, then, otherwise) => {
                    exprs.push(*otherwise);
                    exprs.push(*then);
                }
                ExprKind::Match {
                    arms, otherwise, ..
                } => {
                    exprs.extend(otherwise.iter().copied());
                    exprs.extend(arms.iter().rev().map(|arm| arm.value));
                }
                ExprKind::Index(base, _) => exprs.push(*base),
                ExprKind::Array(items) => exprs.extend(items.iter().rev().copied()),
                _ => match some!(self.val(Query::Expr(expr, outer), need)) {
                    Val::Scalar(node) => roots.push(node),
                    Val::Array(array) => roots.extend(array.elements.iter().copied()),
                    Val::Error => return Ok(Some(Vec::new())),
                },
            }
        }
        let mut candidates = Vec::new();
        let mut stack: Vec<NodeId> = roots.into_iter().rev().collect();
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::PathsWrittenDestination.step();
            match &self.node(node)?.op {
                Op::NodeRef(item) => {
                    if !candidates.contains(item) {
                        candidates.push(*item);
                    }
                }
                Op::Use(inner) | Op::Chosen(inner) | Op::Annotate(_, inner) => stack.push(*inner),
                Op::If(_, then, otherwise) => {
                    stack.push(*otherwise);
                    stack.push(*then);
                }
                Op::Pick { items, .. } => stack.extend(items.iter().rev().copied()),
                Op::Match {
                    arms, otherwise, ..
                } => {
                    stack.extend(otherwise.iter().copied());
                    stack.extend(arms.iter().rev().map(|(_, value)| *value));
                }
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Bare(_)
                | Op::Pi
                | Op::Read(_)
                | Op::Arg(_)
                | Op::Slot(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::Element { .. }
                | Op::Array(_)
                | Op::Convert(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::For(_)
                | Op::ToRead { .. }
                | Op::Join(..)
                | Op::Slice { .. }
                | Op::History { .. }
                | Op::Stepped(_)
                | Op::Native { .. } => return Ok(Some(Vec::new())),
            }
        }
        Ok(Some(candidates))
    }

    fn enclosing_wire(&self, scope: ScopeId) -> Result<Option<WireId>, Internal> {
        let mut current = Some(scope);
        while let Some(id) = current {
            crate::semantics::counted_walk::CountedWalk::PathsEnclosingWire.step();
            let scope = self.scope_of(id)?;
            if let ScopeKind::WireClause { wire } = scope.kind {
                return Ok(Some(wire));
            }
            current = scope.parent;
        }
        Ok(None)
    }

    pub(in crate::semantics::analysis) fn index(
        &mut self,
        base: ExprId,
        index: ExprId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        match self.bound_element(base, index, scope, need)? {
            Shortcut::Pending => return Ok(None),
            Shortcut::Read(val) => return Ok(Some(val)),
            Shortcut::Declined => {}
        }
        let base_val = self.val(Query::Expr(base, scope), need);
        let index_val = self.val(Query::Expr(index, scope), need);
        let (base_val, index_val) = (some!(base_val), some!(index_val));
        self.single_operand(
            &index_val,
            Code::IndexType,
            "an index is an integer, not an array",
        );
        let index_node = match index_val {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(
                    Code::IndexType,
                    loc,
                    scope,
                    "an index is an integer, not an array",
                )?;
                return Ok(Some(Val::Error));
            }
            Val::Error => return Ok(Some(Val::Error)),
        };
        self.index_val(base, base_val, index_node, loc, scope, need)
    }

    pub(in crate::semantics::analysis) fn index_from_end(
        &mut self,
        base: ExprId,
        offset: ExprId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let base_val = self.val(Query::Expr(base, scope), need);
        let offset_val = self.val(Query::Expr(offset, scope), need);
        let (base_val, offset_val) = (some!(base_val), some!(offset_val));
        self.single_operand(
            &offset_val,
            Code::IndexType,
            "an index is an integer, not an array",
        );
        let offset_node = match offset_val {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(
                    Code::IndexType,
                    loc,
                    scope,
                    "an index is an integer, not an array",
                )?;
                return Ok(Some(Val::Error));
            }
            Val::Error => return Ok(Some(Val::Error)),
        };
        if matches!(base_val, Val::Error) {
            return Ok(Some(Val::Error));
        }
        let index_node = match self.memo.from_ends.get(&(offset, scope)) {
            Some(known) => *known,
            None => {
                let created =
                    some!(self.counted_from_end(&base_val, offset_node, loc, scope, need)?);
                self.memo.from_ends.insert((offset, scope), created);
                created
            }
        };
        self.index_val(base, base_val, index_node, loc, scope, need)
    }

    fn index_val(
        &mut self,
        base: ExprId,
        base_val: Val,
        index_node: NodeId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        if let Val::Scalar(base_node) = base_val
            && self.arrays.contains(&base_node)
        {
            let element = Op::Element {
                base: base_node,
                index: index_node,
            };
            return Ok(Some(self.scalar(element, loc, scope)?));
        }
        let array = match &base_val {
            Val::Array(array) => array.clone(),
            Val::Scalar(node) => {
                if !some!(self.sig(*node, need)).is_error() {
                    self.error(
                        Code::NotAnArray,
                        self.hir.expr(base)?.loc,
                        scope,
                        "only an array is indexed",
                    )?;
                }
                return Ok(Some(Val::Error));
            }
            Val::Error => return Ok(Some(Val::Error)),
        };
        let outer = array.shape.outer().ok_or(Internal::missed("array shape"))?;
        let sig = some!(self.sig(index_node, need));
        let evaluated = !sig.reactive && !sig.frame && !sig.is_error();
        let typed_index = matches!(sig.kind, Kind::Int | Kind::Any);
        if evaluated && typed_index {
            let position = match some!(self.cval(Query::Eval(index_node, EvalTy::Exact), need)) {
                CVal::Int(position) => Some(position),
                CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_) => {
                    self.error(
                        Code::IndexType,
                        self.node(index_node)?.loc,
                        scope,
                        "an index is an integer",
                    )?;
                    return Ok(Some(Val::Error));
                }
                CVal::Error => None,
            };
            if let Some(position) = position
                && (0..i128::from(outer)).contains(&position)
            {
                let position =
                    usize::try_from(position).map_err(|_| Internal::missed("array element"))?;
                let chosen = base_val.slice(position)?;
                let group = self.value_group(base, &base_val, scope)?;
                let mut reads = Vec::new();
                for leaf in chosen.nodes() {
                    reads.push(self.chosen_read(leaf, group, loc, scope)?);
                }
                return Ok(Some(self.shaped(chosen.shape(), reads)));
            }
        }
        let rows: Vec<Val> = (0..outer as usize)
            .map(|row| base_val.slice(row))
            .collect::<Result<_, _>>()?;
        let inner = array.shape.inner();
        let group = match self.existing_group(base, &base_val, scope)? {
            None if outer == 0 => some!(self.sliced_group(base, scope, need)?),
            group => group,
        };
        let columns: Vec<Vec<NodeId>> = rows.iter().map(Val::nodes).collect();
        let width = inner.count();
        let elements: Vec<NodeId> = (0..width)
            .map(|position| {
                let items = columns
                    .iter()
                    .map(|column| {
                        column
                            .get(position)
                            .copied()
                            .ok_or(Internal::missed("array element"))
                    })
                    .collect::<Result<_, _>>()?;
                let pick = self.push_node(
                    Op::Pick {
                        index: index_node,
                        items,
                    },
                    loc,
                    scope,
                )?;
                if let Some(group) = group {
                    self.memo.groups.insert(pick, group);
                }
                Ok(pick)
            })
            .collect::<Result<_, Internal>>()?;
        if evaluated {
            self.memo.eager_reads.push((index_node, elements.clone()));
        }
        Ok(Some(self.shaped(inner, elements)))
    }

    fn sliced_group(
        &mut self,
        base: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<NodeId>>, Internal> {
        let ExprKind::Slice { base: sliced, .. } = self.hir.expr(base)?.kind else {
            return Ok(Some(None));
        };
        let val = some!(self.val(Query::Expr(sliced, scope), need));
        if val.nodes().is_empty() {
            return Ok(Some(None));
        }
        Ok(Some(Some(self.value_group(sliced, &val, scope)?)))
    }

    fn bound_element(
        &mut self,
        base: ExprId,
        index: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Shortcut<Val>, Internal> {
        let slice = match self.bound_slice(base, index, scope, need)? {
            Shortcut::Pending => return Ok(Shortcut::Pending),
            Shortcut::Declined => return Ok(Shortcut::Declined),
            Shortcut::Read(slice) => slice,
        };
        let items = slice
            .offset
            .checked_add(slice.shape.count())
            .and_then(|end| slice.array.1.get(slice.offset..end))
            .ok_or(Internal::missed("array element"))?
            .to_vec();
        let group = match slice.query {
            Query::Let(..) | Query::Param(..) => {
                Some(self.bound_group(slice.query, &slice.array, slice.loc)?)
            }
            Query::Input(_)
            | Query::State(_)
            | Query::Expr(..)
            | Query::Attr(..)
            | Query::Destination(_)
            | Query::Sig(_)
            | Query::Eval(..) => None,
        };
        let mut reads = Vec::with_capacity(items.len());
        for item in items {
            let read = self.read_one(item, slice.loc, scope)?;
            if let Some(group) = group {
                self.memo.groups.insert(read, group);
            }
            reads.push(read);
        }
        Ok(Shortcut::Read(self.shaped(slice.shape, reads)))
    }

    fn bound_slice(
        &mut self,
        base: ExprId,
        index: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Shortcut<BoundSlice>, Internal> {
        let hir = self.hir;
        let mut levels = vec![index];
        let mut root = base;
        let below = loop {
            match &hir.expr(root)?.kind {
                ExprKind::Index(inner, position) => {
                    if let Some(known) = self.memo.bound_slices.get(&(*position, scope)) {
                        break known.clone();
                    }
                    levels.push(*position);
                    root = *inner;
                }
                ExprKind::Name(name) => {
                    match self.root_slice(name, hir.expr(root)?.loc, scope, need)? {
                        Shortcut::Pending => return Ok(Shortcut::Pending),
                        Shortcut::Declined => break None,
                        Shortcut::Read(slice) => break Some(slice),
                    }
                }
                ExprKind::Error
                | ExprKind::Number(..)
                | ExprKind::Bool(_)
                | ExprKind::Str(_)
                | ExprKind::Variant { .. }
                | ExprKind::To
                | ExprKind::Convert(..)
                | ExprKind::Array(_)
                | ExprKind::Field(..)
                | ExprKind::IndexFromEnd(..)
                | ExprKind::Slice { .. }
                | ExprKind::History { .. }
                | ExprKind::Stepped(_)
                | ExprKind::Call(..)
                | ExprKind::Unary(..)
                | ExprKind::Binary(..)
                | ExprKind::If(..)
                | ExprKind::Match { .. }
                | ExprKind::For(_) => break None,
            }
        };
        let mut current = below;
        let mut pending = false;
        for position in levels.into_iter().rev() {
            if pending {
                self.request_position(position, scope, need);
                continue;
            }
            let Some(slice) = current.take() else {
                self.memo.bound_slices.insert((position, scope), None);
                continue;
            };
            match self.narrowed(slice, position, scope, need)? {
                Shortcut::Pending => pending = true,
                Shortcut::Declined => {
                    self.memo.bound_slices.insert((position, scope), None);
                }
                Shortcut::Read(next) => {
                    self.memo
                        .bound_slices
                        .insert((position, scope), Some(next.clone()));
                    current = Some(next);
                }
            }
        }
        if pending {
            return Ok(Shortcut::Pending);
        }
        Ok(match current {
            Some(slice) => Shortcut::Read(slice),
            None => Shortcut::Declined,
        })
    }

    fn root_slice(
        &mut self,
        name: &str,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Shortcut<BoundSlice>, Internal> {
        let Found::Query(query) = self.find(name, scope)? else {
            return Ok(Shortcut::Declined);
        };
        let Some(bound) = self.bound(query, need) else {
            return Ok(Shortcut::Pending);
        };
        let BoundVal::Array(array) = bound else {
            return Ok(Shortcut::Declined);
        };
        let mut strides = Vec::new();
        let mut stride = 1usize;
        for length in array.0.lengths() {
            strides.push(stride);
            stride = stride.saturating_mul(usize::try_from(length).unwrap_or(usize::MAX));
        }
        strides.reverse();
        Ok(Shortcut::Read(BoundSlice {
            query,
            loc,
            shape: array.0.clone(),
            array,
            strides: strides.into(),
            level: 0,
            offset: 0,
        }))
    }

    fn request_position(&self, position: ExprId, scope: ScopeId, need: &mut Vec<Query>) {
        let Some(Val::Scalar(node)) = self.val(Query::Expr(position, scope), need) else {
            return;
        };
        if let Some(sig) = self.sig(node, need)
            && !sig.reactive
            && !sig.frame
            && sig.kind == Kind::Int
        {
            self.cval(Query::Eval(node, EvalTy::Exact), need);
        }
    }

    fn narrowed(
        &mut self,
        slice: BoundSlice,
        position: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Shortcut<BoundSlice>, Internal> {
        let Some(val) = self.val(Query::Expr(position, scope), need) else {
            return Ok(Shortcut::Pending);
        };
        let (Val::Scalar(node), Some(length)) = (val, slice.shape.outer()) else {
            return Ok(Shortcut::Declined);
        };
        let Some(sig) = self.sig(node, need) else {
            return Ok(Shortcut::Pending);
        };
        if sig.is_error() || sig.reactive || sig.frame || sig.kind != Kind::Int {
            return Ok(Shortcut::Declined);
        }
        let Some(value) = self.cval(Query::Eval(node, EvalTy::Exact), need) else {
            return Ok(Shortcut::Pending);
        };
        let CVal::Int(element) = value else {
            return Ok(Shortcut::Declined);
        };
        if !(0..i128::from(length)).contains(&element) {
            return Ok(Shortcut::Declined);
        }
        let stride = slice
            .strides
            .get(slice.level)
            .copied()
            .ok_or(Internal::missed("array stride"))?;
        let offset = usize::try_from(element)
            .ok()
            .and_then(|element| element.checked_mul(stride))
            .and_then(|step| step.checked_add(slice.offset))
            .ok_or(Internal::missed("array element"))?;
        Ok(Shortcut::Read(BoundSlice {
            level: slice.level + 1,
            offset,
            shape: slice.shape.inner(),
            ..slice
        }))
    }

    pub(in crate::semantics::analysis) fn chosen_read(
        &mut self,
        leaf: NodeId,
        group: NodeId,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<NodeId, Internal> {
        let op = match self.node(leaf)?.op {
            Op::Read(binding) => Op::Read(binding),
            _ => Op::Chosen(leaf),
        };
        let read = self.push_node(op, loc, scope)?;
        self.memo.groups.insert(read, group);
        Ok(read)
    }

    pub(in crate::semantics::analysis) fn bound_group(
        &mut self,
        query: Query,
        array: &(Shape, Vec<Bound>),
        read: Loc,
    ) -> Result<NodeId, Internal> {
        if let Some(group) = self.memo.bound_groups.get(&query) {
            return Ok(*group);
        }
        if let Some(group) = self
            .bound_value(query)?
            .and_then(|value| self.memo.literal_groups.get(&value).copied())
        {
            self.memo.bound_groups.insert(query, group);
            return Ok(group);
        }
        let (loc, scope) = match query {
            Query::Let(scope, local) => match self.let_decl(scope, local)? {
                Some(decl) => (self.hir.expr(decl.value)?.loc, self.let_scope(scope, local)),
                None => (read, scope),
            },
            Query::Param(scope, index) => {
                (self.param_value_loc(scope, index)?.unwrap_or(read), scope)
            }
            Query::Input(_)
            | Query::State(_)
            | Query::Expr(..)
            | Query::Attr(..)
            | Query::Destination(_)
            | Query::Sig(_)
            | Query::Eval(..) => return Err(Internal::missed("array binding")),
        };
        let mut elements = Vec::with_capacity(array.1.len());
        for item in &array.1 {
            elements.push(match item {
                Bound::Value(node) => *node,
                Bound::Cell(binding) => self.push_node(Op::Read(*binding), loc, scope)?,
            });
        }
        let group = self.push_node(Op::Array(elements), loc, scope)?;
        self.memo.bound_groups.insert(query, group);
        self.memo.group_list.push(group);
        Ok(group)
    }

    pub(super) fn bound_value(&self, query: Query) -> Result<Option<(ExprId, ScopeId)>, Internal> {
        Ok(match query {
            Query::Let(scope, local) => self
                .let_decl(scope, local)?
                .map(|decl| (decl.value, self.let_scope(scope, local))),
            Query::Param(scope, index) => {
                let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
                    return Err(Internal::missed("locals scope"));
                };
                let params = self.owner_locals(locals.owner)?.0;
                match locals.params.get(index as usize) {
                    Some(ParamSource::Arg { expr, scope }) => Some((*expr, *scope)),
                    Some(ParamSource::Default) => params
                        .get(index as usize)
                        .and_then(|param| param.default)
                        .zip(self.memo.prefix_scopes.get(&(scope, index)).copied()),
                    Some(ParamSource::Missing | ParamSource::Bound(_)) | None => None,
                }
            }
            Query::Input(_)
            | Query::State(_)
            | Query::Expr(..)
            | Query::Attr(..)
            | Query::Destination(_)
            | Query::Sig(_)
            | Query::Eval(..) => None,
        })
    }

    fn param_value_loc(&self, scope: ScopeId, index: u32) -> Result<Option<Loc>, Internal> {
        let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
            return Err(Internal::missed("locals scope"));
        };
        let params = self.owner_locals(locals.owner)?.0;
        let expr = match locals.params.get(index as usize) {
            Some(ParamSource::Arg { expr, .. }) => Some(*expr),
            Some(ParamSource::Default) => {
                params.get(index as usize).and_then(|param| param.default)
            }
            Some(ParamSource::Missing | ParamSource::Bound(_)) | None => None,
        };
        expr.map(|expr| Ok(self.hir.expr(expr)?.loc)).transpose()
    }

    pub(in crate::semantics::analysis) fn value_group(
        &mut self,
        base: ExprId,
        val: &Val,
        scope: ScopeId,
    ) -> Result<NodeId, Internal> {
        if let Some(group) = self.existing_group(base, val, scope)? {
            return Ok(group);
        }
        let loc = self.hir.expr(base)?.loc;
        let group = self.push_node(Op::Array(val.nodes()), loc, scope)?;
        self.memo.group_list.push(group);
        Ok(group)
    }

    fn existing_group(
        &self,
        base: ExprId,
        val: &Val,
        scope: ScopeId,
    ) -> Result<Option<NodeId>, Internal> {
        if let Some(group) = self.memo.literal_groups.get(&(base, scope)) {
            return Ok(Some(*group));
        }
        if !matches!(
            self.hir.expr(base)?.kind,
            ExprKind::Index(..) | ExprKind::Name(_)
        ) {
            return Ok(None);
        }
        Ok(val
            .nodes()
            .first()
            .and_then(|leaf| self.memo.groups.get(leaf))
            .copied())
    }

    pub(in crate::semantics::analysis) fn run_destination(
        &mut self,
        wire: WireId,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let (expr, scope) = {
            let record = self.wire(wire)?;
            (self.hir.wire(record.wire)?.destination, record.scope)
        };
        let Some(val) = self.val(Query::Expr(expr, scope), need) else {
            return Ok(false);
        };
        let loc = self.hir.expr(expr)?.loc;
        let node = match val {
            Val::Scalar(node) if !self.arrays.contains(&node) => node,
            Val::Scalar(_) | Val::Array(_) => {
                self.error(
                    Code::DestinationType,
                    loc,
                    scope,
                    "a destination is one instance, not an array",
                )?;
                self.memo.destinations.insert(wire, Destination::Error);
                return Ok(true);
            }
            Val::Error => {
                self.memo.destinations.insert(wire, Destination::Error);
                return Ok(true);
            }
        };
        let Some(sig) = self.sig(node, need) else {
            return Ok(false);
        };
        if sig.is_error() {
            self.memo.destinations.insert(wire, Destination::Error);
            return Ok(true);
        }
        if sig.kind != Kind::Node {
            self.error(
                Code::DestinationType,
                loc,
                scope,
                "a destination names an instance",
            )?;
            self.memo.destinations.insert(wire, Destination::Error);
            return Ok(true);
        }
        let destination = if sig.reactive {
            Destination::Select {
                node,
                candidates: self.candidates(node)?,
            }
        } else {
            match self.cval(Query::Eval(node, EvalTy::Node), need) {
                None => return Ok(false),
                Some(CVal::Node(item)) => Destination::Fixed(item),
                Some(_) => Destination::Error,
            }
        };
        if let Destination::Fixed(item @ ItemRef::Use(_)) = destination {
            if self.stands_for_nothing_known(item)? {
                self.memo.destinations.insert(wire, Destination::Error);
                return Ok(true);
            }
            self.error(
                Code::DestinationType,
                loc,
                scope,
                "a destination names an instance, not a pattern use",
            )?;
            self.memo.destinations.insert(wire, Destination::Error);
            return Ok(true);
        }
        self.memo.destinations.insert(wire, destination);
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn candidates(
        &self,
        root: NodeId,
    ) -> Result<Vec<ItemRef>, Internal> {
        let mut found = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::PathsCandidates.step();
            match &self.node(node)?.op {
                Op::NodeRef(item) => found.push(*item),
                Op::If(_, then, otherwise) => {
                    stack.push(*otherwise);
                    stack.push(*then);
                }
                Op::Pick { items, .. } => stack.extend(items.iter().rev().copied()),
                Op::Match {
                    arms, otherwise, ..
                } => {
                    stack.extend(otherwise.iter().copied());
                    stack.extend(arms.iter().rev().map(|(_, value)| *value));
                }
                Op::Annotate(_, inner) | Op::Use(inner) | Op::Chosen(inner) => stack.push(*inner),
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Bare(_)
                | Op::Pi
                | Op::Read(_)
                | Op::Arg(_)
                | Op::Slot(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::Convert(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::For(_)
                | Op::Element { .. }
                | Op::Array(_)
                | Op::ToRead { .. }
                | Op::Join(..)
                | Op::Slice { .. }
                | Op::History { .. }
                | Op::Stepped(_)
                | Op::Native { .. } => {}
            }
        }
        Ok(found)
    }

    pub(in crate::semantics::analysis) fn through_uses(
        &self,
        node: NodeId,
    ) -> Result<NodeId, Internal> {
        let mut path = Vec::new();
        let mut current = node;
        let end = loop {
            if let Some(end) = self.memo.use_ends.borrow().get(&current).copied() {
                break end;
            }
            self.memo.use_steps.set(self.memo.use_steps.get() + 1);
            if self.memo.frame_reads.contains_key(&current) {
                break current;
            }
            let Op::Use(inner) = self.node(current)?.op else {
                break current;
            };
            path.push(current);
            current = inner;
        };
        self.memo
            .use_ends
            .borrow_mut()
            .extend(path.into_iter().map(|step| (step, end)));
        Ok(end)
    }

    pub(in crate::semantics::analysis) fn attr_named(
        &self,
        item: ItemRef,
        name: &str,
    ) -> Result<Option<AttrId>, Internal> {
        let attrs = match self.kind_attrs(item)? {
            Some(attrs) => attrs,
            None => return Ok(None),
        };
        Ok(attrs
            .iter()
            .position(|attr| attr.name == name)
            .map(|position| AttrId(u32::try_from(position).unwrap_or(u32::MAX))))
    }

    pub(in crate::semantics::analysis) fn attr_decl(
        &self,
        item: ItemRef,
        attr: AttrId,
    ) -> Result<Option<&AttrDecl>, Internal> {
        let attrs = match self.kind_attrs(item)? {
            Some(attrs) => attrs,
            None => return Ok(None),
        };
        attrs
            .get(attr.index())
            .map(Some)
            .ok_or(Internal::missed("attribute"))
    }

    fn kind_attrs(&self, item: ItemRef) -> Result<Option<&[AttrDecl]>, Internal> {
        Ok(match item {
            ItemRef::Instance(instance) => {
                Some(&self.node_decl(self.instance(instance)?.kind)?.attrs)
            }
            ItemRef::Wire(wire) => Some(&self.wire_decl(self.wire(wire)?.kind)?.attrs),
            ItemRef::Use(_) => None,
        })
    }

    pub(in crate::semantics::analysis) fn attr_sources(
        &self,
        item: ItemRef,
    ) -> Result<&[AttrSource], Internal> {
        Ok(match item {
            ItemRef::Instance(instance) => self.instance(instance)?.attrs.as_slice(),
            ItemRef::Wire(wire) => self.wire(wire)?.attrs.as_slice(),
            ItemRef::Use(_) => &[],
        })
    }
}

const CHOSEN_PATHS: &str =
    "the references this path chooses among hold attributes of different shapes";
