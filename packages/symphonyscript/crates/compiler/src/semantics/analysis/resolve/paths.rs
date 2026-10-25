use symphonyscript_core::{AttrDecl, AttrId};

use super::found::Found;
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
use crate::semantics::analysis::ids::{NodeId, ScopeId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::shape::Shape;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Ident, Loc};

enum Shortcut<T> {
    Pending,
    Declined,
    Read(T),
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
        match &self.hir.expr(base)?.kind {
            ExprKind::To => return self.destination_read(name, loc, scope, need),
            ExprKind::Field(inner, _) if matches!(self.hir.expr(*inner)?.kind, ExprKind::To) => {
                return Ok(Some(Val::Error));
            }
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
            | ExprKind::Call(..)
            | ExprKind::Unary(..)
            | ExprKind::Binary(..)
            | ExprKind::If(..)
            | ExprKind::Match { .. } => {}
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
            Val::Error => return Ok(Some(Val::Error)),
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
            | Op::ToRead { .. } => {
                let sig = some!(self.sig(node, need));
                if sig.is_error() {
                    return Ok(Some(Val::Error));
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
        self.step(item, name, loc, scope, need)
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
            None if self.names_unknown(item)? => Some(Val::Error),
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

    fn destination_read(
        &mut self,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let Some(wire) = self.enclosing_wire(scope)? else {
            return Ok(Some(Val::Error));
        };
        let destination = some!(self.destination(Query::Destination(wire), need));
        let mut attrs = Vec::new();
        for candidate in destination.candidates() {
            match self.attr_named(candidate, &name.text)? {
                Some(attr) => attrs.push((candidate, attr)),
                None => {
                    let owner = self.item_description(candidate)?;
                    self.error(
                        Code::UnknownAttribute,
                        name.loc,
                        scope,
                        format!("the destination {owner} has no attribute `{}`", name.text),
                    )?;
                    return Ok(Some(Val::Error));
                }
            }
        }
        if attrs.is_empty() {
            return Ok(Some(Val::Error));
        }
        Ok(Some(self.scalar(
            Op::ToRead {
                wire,
                attr: name.text.clone(),
                attrs,
            },
            loc,
            scope,
        )?))
    }

    fn enclosing_wire(&self, scope: ScopeId) -> Result<Option<WireId>, Internal> {
        let mut current = Some(scope);
        while let Some(id) = current {
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
            Val::Scalar(_) => {
                self.error(
                    Code::NotAnArray,
                    self.hir.expr(base)?.loc,
                    scope,
                    "only an array is indexed",
                )?;
                return Ok(Some(Val::Error));
            }
            Val::Error => return Ok(Some(Val::Error)),
        };
        let outer = array.shape.outer().ok_or(Internal::missed("array shape"))?;
        let sig = some!(self.sig(index_node, need));
        if sig.is_error() {
            return Ok(Some(Val::Error));
        }
        if !sig.reactive && !sig.frame {
            if sig.kind != Kind::Int {
                self.error(
                    Code::IndexType,
                    self.node(index_node)?.loc,
                    scope,
                    "an index is an integer",
                )?;
                return Ok(Some(Val::Error));
            }
            let position = match some!(self.cval(Query::Eval(index_node, EvalTy::Exact), need)) {
                CVal::Int(position) => position,
                CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_)
                | CVal::Error => return Ok(Some(Val::Error)),
            };
            if (0..i128::from(outer)).contains(&position) {
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
        let group = self.existing_group(base, &base_val, scope)?;
        let columns: Vec<Vec<NodeId>> = rows.iter().map(Val::nodes).collect();
        let width = columns.first().map_or(0, Vec::len);
        let elements = (0..width)
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
        Ok(Some(self.shaped(inner, elements)))
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
                | ExprKind::Call(..)
                | ExprKind::Unary(..)
                | ExprKind::Binary(..)
                | ExprKind::If(..)
                | ExprKind::Match { .. } => break None,
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
                Some(decl) => (self.hir.expr(decl.value)?.loc, scope),
                None => (read, scope),
            },
            Query::Param(scope, index) => {
                (self.param_value_loc(scope, index)?.unwrap_or(read), scope)
            }
            Query::Input(_)
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
            Query::Let(scope, local) => {
                self.let_decl(scope, local)?.map(|decl| (decl.value, scope))
            }
            Query::Param(scope, index) => {
                let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
                    return Err(Internal::missed("locals scope"));
                };
                let params = match locals.owner {
                    LocalsOwner::Fn(function) => &self.hir.fn_decl(function)?.params,
                    LocalsOwner::Pattern(pattern) => &self.hir.pattern(pattern)?.params,
                };
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
        let params = match locals.owner {
            LocalsOwner::Fn(function) => &self.hir.fn_decl(function)?.params,
            LocalsOwner::Pattern(pattern) => &self.hir.pattern(pattern)?.params,
        };
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
            Val::Scalar(node) => node,
            Val::Array(_) => {
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
            if self.names_unknown(item)? {
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
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::Convert(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::Element { .. }
                | Op::Array(_)
                | Op::ToRead { .. } => {}
            }
        }
        Ok(found)
    }

    pub(in crate::semantics::analysis) fn through_uses(
        &self,
        mut node: NodeId,
    ) -> Result<NodeId, Internal> {
        while let Op::Use(inner) = self.node(node)?.op {
            node = inner;
        }
        Ok(node)
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
