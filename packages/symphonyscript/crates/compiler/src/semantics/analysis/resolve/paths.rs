use symphonyscript_core::{AttrDecl, AttrId};

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::attr_source::AttrSource;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::{NodeId, ScopeId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
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
                return Ok(Some(base_val.slice(position)?));
            }
        }
        let rows: Vec<Val> = (0..outer as usize)
            .map(|row| base_val.slice(row))
            .collect::<Result<_, _>>()?;
        let inner = array.shape.inner();
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
                self.push_node(
                    Op::Pick {
                        index: index_node,
                        items,
                    },
                    loc,
                    scope,
                )
            })
            .collect::<Result<_, _>>()?;
        Ok(Some(self.shaped(inner, elements)))
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
        if let Destination::Fixed(ItemRef::Use(_)) = destination {
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
                Op::Annotate(_, inner) | Op::Use(inner) => stack.push(*inner),
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
