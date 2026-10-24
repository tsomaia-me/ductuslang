use symphonyscript_core::{AttrDecl, AttrId};

use crate::diagnostics::Code;
use crate::semantics::analysis::attr_source::AttrSource;
use crate::semantics::analysis::cval::CVal;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::{NodeId, ScopeId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::node::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope::ScopeKind;
use crate::semantics::analysis::sig::Kind;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Ident, Loc};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn field(
        &mut self,
        base: ExprId,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        match &self.hir.expr(base).kind {
            ExprKind::To => return self.destination_read(name, loc, scope, need),
            ExprKind::Field(inner, _) if matches!(self.hir.expr(*inner).kind, ExprKind::To) => {
                return Some(Val::Error);
            }
            _ => {}
        }
        let base_val = self.val(Query::Expr(base, scope), need)?;
        let node = match base_val {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(
                    Code::UnknownAttribute,
                    loc,
                    scope,
                    "an array has no attributes; index it first",
                );
                return Some(Val::Error);
            }
            Val::Error => return Some(Val::Error),
        };
        let item = match &self.node(self.through_uses(node)).op {
            Op::NodeRef(item) => *item,
            Op::Bare(_) => return Some(Val::Error),
            _ => {
                let sig = self.sig(node, need)?;
                if sig.is_error() {
                    return Some(Val::Error);
                }
                if sig.kind != Kind::Node {
                    self.error(
                        Code::UnknownAttribute,
                        loc,
                        scope,
                        format!("`.{}` reads an attribute of an instance or a wire, and this is a value", name.text),
                    );
                    return Some(Val::Error);
                }
                if sig.reactive {
                    self.error(
                        Code::RuntimePath,
                        loc,
                        scope,
                        "a path starts at an instance chosen at compile time",
                    );
                    return Some(Val::Error);
                }
                match self.cval(Query::Eval(node, EvalTy::Node), need)? {
                    CVal::Node(item) => item,
                    _ => return Some(Val::Error),
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
    ) -> Option<Val> {
        if let Some(child) = self.child_named(item, &name.text) {
            return Some(self.scalar(Op::NodeRef(child), loc, scope));
        }
        match self.attr_named(item, &name.text) {
            Some(attr) => {
                let bound = self.bound(Query::Attr(item, attr), need)?;
                Some(self.read(&bound, loc, scope))
            }
            None => {
                let owner = self.item_description(item);
                self.error(
                    Code::UnknownAttribute,
                    name.loc,
                    scope,
                    format!("{owner} has no child, wire or attribute `{}`", name.text),
                );
                Some(Val::Error)
            }
        }
    }

    fn destination_read(
        &mut self,
        name: &Ident,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        let Some(wire) = self.enclosing_wire(scope) else {
            return Some(Val::Error);
        };
        let destination = self.destination(Query::Destination(wire), need)?;
        let mut attrs = Vec::new();
        for candidate in destination.candidates() {
            match self.attr_named(candidate, &name.text) {
                Some(attr) => attrs.push((candidate, attr)),
                None => {
                    let owner = self.item_description(candidate);
                    self.error(
                        Code::UnknownAttribute,
                        name.loc,
                        scope,
                        format!("the destination {owner} has no attribute `{}`", name.text),
                    );
                    return Some(Val::Error);
                }
            }
        }
        if attrs.is_empty() {
            return Some(Val::Error);
        }
        Some(self.scalar(
            Op::ToRead {
                wire,
                attr: name.text.clone(),
                attrs,
            },
            loc,
            scope,
        ))
    }

    fn enclosing_wire(&self, scope: ScopeId) -> Option<WireId> {
        let mut current = Some(scope);
        while let Some(id) = current {
            let scope = self.scope_of(id);
            if let ScopeKind::WireClause { wire } = scope.kind {
                return Some(wire);
            }
            current = scope.parent;
        }
        None
    }

    pub(in crate::semantics::analysis) fn index(
        &mut self,
        base: ExprId,
        index: ExprId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        let base_val = self.val(Query::Expr(base, scope), need);
        let index_val = self.val(Query::Expr(index, scope), need);
        let (base_val, index_val) = (base_val?, index_val?);
        let index_node = match index_val {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(
                    Code::IndexType,
                    loc,
                    scope,
                    "an index is an integer, not an array",
                );
                return Some(Val::Error);
            }
            Val::Error => return Some(Val::Error),
        };
        let array = match &base_val {
            Val::Array(array) => array.clone(),
            Val::Scalar(_) => {
                self.error(
                    Code::NotAnArray,
                    self.hir.expr(base).loc,
                    scope,
                    "only an array is indexed",
                );
                return Some(Val::Error);
            }
            Val::Error => return Some(Val::Error),
        };
        let outer = array.shape.outer().unwrap_or(0);
        let sig = self.sig(index_node, need)?;
        if sig.is_error() {
            return Some(Val::Error);
        }
        if !sig.reactive && !sig.frame {
            if sig.kind != Kind::Int {
                self.error(
                    Code::IndexType,
                    self.node(index_node).loc,
                    scope,
                    "an index is an integer",
                );
                return Some(Val::Error);
            }
            let position = match self.cval(Query::Eval(index_node, EvalTy::Exact), need)? {
                CVal::Int(position) => position,
                _ => return Some(Val::Error),
            };
            if (0..i128::from(outer)).contains(&position) {
                return Some(base_val.slice(usize::try_from(position).unwrap_or(0)));
            }
        }
        let rows: Vec<Val> = (0..outer as usize).map(|row| base_val.slice(row)).collect();
        let inner = array.shape.inner();
        let columns: Vec<Vec<NodeId>> = rows.iter().map(Val::nodes).collect();
        let width = columns.first().map_or(0, Vec::len);
        let elements = (0..width)
            .map(|position| {
                let items = columns
                    .iter()
                    .filter_map(|column| column.get(position).copied())
                    .collect();
                self.push_node(
                    Op::Pick {
                        index: index_node,
                        items,
                    },
                    loc,
                    scope,
                )
            })
            .collect();
        Some(self.shaped(inner, elements))
    }

    pub(in crate::semantics::analysis) fn run_destination(
        &mut self,
        wire: WireId,
        need: &mut Vec<Query>,
    ) -> bool {
        let (expr, scope) = {
            let record = self.wire(wire);
            (self.hir.wire(record.wire).destination, record.scope)
        };
        let Some(val) = self.val(Query::Expr(expr, scope), need) else {
            return false;
        };
        let loc = self.hir.expr(expr).loc;
        let node = match val {
            Val::Scalar(node) => node,
            Val::Array(_) => {
                self.error(
                    Code::DestinationType,
                    loc,
                    scope,
                    "a destination is one instance, not an array",
                );
                self.memo.destinations.insert(wire, Destination::Error);
                return true;
            }
            Val::Error => {
                self.memo.destinations.insert(wire, Destination::Error);
                return true;
            }
        };
        let Some(sig) = self.sig(node, need) else {
            return false;
        };
        if sig.is_error() {
            self.memo.destinations.insert(wire, Destination::Error);
            return true;
        }
        if sig.kind != Kind::Node {
            self.error(
                Code::DestinationType,
                loc,
                scope,
                "a destination names an instance",
            );
            self.memo.destinations.insert(wire, Destination::Error);
            return true;
        }
        let destination = if sig.reactive {
            Destination::Select {
                node,
                candidates: self.candidates(node),
            }
        } else {
            match self.cval(Query::Eval(node, EvalTy::Node), need) {
                None => return false,
                Some(CVal::Node(item)) => Destination::Fixed(item),
                Some(_) => Destination::Error,
            }
        };
        if let Destination::Fixed(ItemRef::Use(_)) = destination {
            self.error(
                Code::DestinationType,
                loc,
                scope,
                "a destination names an instance, not a pattern use",
            );
            self.memo.destinations.insert(wire, Destination::Error);
            return true;
        }
        self.memo.destinations.insert(wire, destination);
        true
    }

    pub(in crate::semantics::analysis) fn candidates(&self, root: NodeId) -> Vec<ItemRef> {
        let mut found = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            match &self.node(node).op {
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
                Op::Annotate(_, inner) | Op::Use(inner) => stack.push(*inner),
                _ => {}
            }
        }
        found
    }

    pub(in crate::semantics::analysis) fn through_uses(&self, mut node: NodeId) -> NodeId {
        while let Op::Use(inner) = self.node(node).op {
            node = inner;
        }
        node
    }

    pub(in crate::semantics::analysis) fn attr_named(
        &self,
        item: ItemRef,
        name: &str,
    ) -> Option<AttrId> {
        let attrs: &[AttrDecl] = match item {
            ItemRef::Instance(instance) => {
                &self
                    .vocabulary
                    .node_kind(self.instance(instance).kind)?
                    .attrs
            }
            ItemRef::Wire(wire) => &self.vocabulary.wire_kind(self.wire(wire).kind)?.attrs,
            ItemRef::Use(_) => return None,
        };
        attrs
            .iter()
            .position(|attr| attr.name == name)
            .map(|position| AttrId(u32::try_from(position).unwrap_or(u32::MAX)))
    }

    pub(in crate::semantics::analysis) fn attr_decl(
        &self,
        item: ItemRef,
        attr: AttrId,
    ) -> Option<&AttrDecl> {
        let attrs: &[AttrDecl] = match item {
            ItemRef::Instance(instance) => {
                &self
                    .vocabulary
                    .node_kind(self.instance(instance).kind)?
                    .attrs
            }
            ItemRef::Wire(wire) => &self.vocabulary.wire_kind(self.wire(wire).kind)?.attrs,
            ItemRef::Use(_) => return None,
        };
        attrs.get(attr.index())
    }

    pub(in crate::semantics::analysis) fn attr_sources(&self, item: ItemRef) -> &[AttrSource] {
        match item {
            ItemRef::Instance(instance) => &self.instance(instance).attrs,
            ItemRef::Wire(wire) => &self.wire(wire).attrs,
            ItemRef::Use(_) => &[],
        }
    }
}
