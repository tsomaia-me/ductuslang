use symphonyscript_core::{BinaryOp, Scalar, Ty, UnaryOp};

use super::position::Position;
use super::role::Role;
use crate::semantics::Internal;
use crate::semantics::analysis::attr_value_source::AttrValueSource;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::ids::{InstanceId, NodeId, ScopeId, WireId};
use crate::semantics::analysis::is_float::is_float;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Top};

impl Engine<'_> {
    pub(super) fn declaration_positions(&mut self) -> Result<Vec<Position>, Internal> {
        let mut positions = Vec::new();
        let hir = self.hir;
        for (position, top) in hir.top.iter().enumerate() {
            if hir.is_prelude_top(position) {
                continue;
            }
            match *top {
                Top::Input(input) => {
                    self.demand(Query::Input(input))?;
                    if let Some(position) = self.input_position(input)? {
                        positions.push(position);
                    }
                }
                Top::Let(id) => self.name_let(id)?,
                Top::State(id) => self.demand(Query::State(id))?,
                Top::Fn(_) | Top::Pattern(_) | Top::Placement(_) => {}
            }
        }
        Ok(positions)
    }

    pub(super) fn positions(
        &mut self,
        mut positions: Vec<Position>,
    ) -> Result<Vec<Position>, Internal> {
        for index in 0..self.instances.len() {
            let instance = InstanceId(u32::try_from(index).unwrap_or(u32::MAX));
            self.item_positions(ItemRef::Instance(instance), &mut positions)?;
        }
        for index in 0..self.wires.len() {
            let wire = WireId(u32::try_from(index).unwrap_or(u32::MAX));
            self.demand(Query::Destination(wire))?;
            self.item_positions(ItemRef::Wire(wire), &mut positions)?;
        }
        for (expr, scope) in self.gates.clone() {
            positions.push(self.gate_position(expr, scope)?);
        }
        let mut scope = 0;
        while scope < self.scopes.len() {
            let id = ScopeId(u32::try_from(scope).unwrap_or(u32::MAX));
            if let ScopeKind::Locals(locals) = &self.scope_of(id)?.kind {
                let (params, lets) = match locals.owner {
                    LocalsOwner::Fn(function) => {
                        let decl = self.hir.fn_decl(function)?;
                        (decl.params.len(), decl.lets.len())
                    }
                    LocalsOwner::Pattern(pattern) => {
                        let decl = self.hir.pattern(pattern)?;
                        (decl.params.len(), decl.lets.len())
                    }
                };
                for index in 0..params {
                    self.demand(Query::Param(id, u32::try_from(index).unwrap_or(u32::MAX)))?;
                }
                for index in 0..lets {
                    self.demand(Query::Let(
                        id,
                        LetRef::Local(u32::try_from(index).unwrap_or(u32::MAX)),
                    ))?;
                }
            }
            scope += 1;
        }
        Ok(positions)
    }

    pub(super) fn carried_input_positions(&mut self) -> Result<Vec<Position>, Internal> {
        let hir = self.hir;
        let mut positions = Vec::new();
        for (position, top) in hir.top.iter().enumerate() {
            let Top::Input(input) = *top else {
                continue;
            };
            if hir.is_prelude_top(position) {
                continue;
            }
            let decl = hir.input(input)?;
            let val = self.demand_val(Query::Expr(decl.value, self.source_global))?;
            let carried = match val.nodes().first() {
                Some(node) => self.demand_sig(*node)?.kind == Kind::Bare,
                None => false,
            };
            if !carried {
                continue;
            }
            if let Some(position) = self.input_position(input)? {
                positions.push(position);
            }
        }
        Ok(positions)
    }

    fn input_position(
        &mut self,
        input: crate::semantics::ir::InputId,
    ) -> Result<Option<Position>, Internal> {
        let decl = self.hir.input(input)?;
        let global = self.source_global;
        let found = self.demand_val(Query::Expr(decl.value, global))?;
        let val = self.as_converted(found);
        let bound = self
            .memo
            .inputs
            .get(&input)
            .cloned()
            .ok_or(Internal::missed("demanded input binding"))?;
        let first = match &bound {
            BoundVal::Scalar(crate::semantics::analysis::bound::Bound::Cell(binding)) => *binding,
            BoundVal::Array(array) => match array.1.first() {
                Some(crate::semantics::analysis::bound::Bound::Cell(binding)) => *binding,
                Some(crate::semantics::analysis::bound::Bound::Value(_)) | None => return Ok(None),
            },
            BoundVal::Scalar(crate::semantics::analysis::bound::Bound::Value(_))
            | BoundVal::Error => return Ok(None),
        };
        let scalar = some!(self.input_type(first)?);
        let mut ty = Ty::of(scalar);
        if let Val::Array(array) = &val {
            ty.lengths = array.shape.lengths();
        }
        Ok(Some(Position {
            val,
            ty,
            unit: None,
            role: Role::Initializer,
            name: None,
            loc: self.hir.expr(decl.value)?.loc,
            scope: global,
            expr: decl.value,
        }))
    }

    pub(crate) fn input_type(
        &mut self,
        binding: crate::semantics::analysis::ids::BindingId,
    ) -> Result<Option<symphonyscript_core::Scalar>, Internal> {
        let record = self.binding(binding)?.clone();
        if let Some(annotation) = record.annotation {
            return Ok(Some(annotation));
        }
        let sig = self.demand_sig(record.init)?;
        Ok(match self.cell_kind(&record, sig.kind) {
            Kind::Int => Some(symphonyscript_core::Scalar::I64),
            Kind::Float => Some(symphonyscript_core::Scalar::F64),
            Kind::Bool => Some(symphonyscript_core::Scalar::Bool),
            Kind::Enum(id) => Some(symphonyscript_core::Scalar::Enum(id)),
            Kind::Bare => self.typing.own.get(&binding).copied(),
            Kind::Str | Kind::Node | Kind::Any | Kind::Error => None,
        })
    }

    fn item_positions(
        &mut self,
        item: ItemRef,
        positions: &mut Vec<Position>,
    ) -> Result<(), Internal> {
        let sources = self.attr_sources(item)?.to_vec();
        for source in sources {
            let Some(decl) = self.attr_decl(item, source.attr)?.cloned() else {
                continue;
            };
            self.demand(Query::Attr(item, source.attr))?;
            if let AttrValueSource::Expr(expr, scope) = source.value {
                let found = self.demand_val(Query::Expr(expr, scope))?;
                let mut val = self.as_converted(found);
                if let Some(group) = self.memo.literal_groups.get(&(expr, scope)).copied()
                    && self.demand_sig(group)?.is_error()
                {
                    val = Val::Error;
                }
                positions.push(Position {
                    val,
                    ty: decl.ty.clone(),
                    unit: decl.unit,
                    role: Role::Attribute,
                    name: Some(decl.name.clone()),
                    loc: self.hir.expr(expr)?.loc,
                    scope,
                    expr,
                });
            }
        }
        let gate = match item {
            ItemRef::Instance(id) => {
                let record = self.instance(id)?;
                self.hir
                    .placement(record.placement)?
                    .gate
                    .map(|gate| (gate, record.scope))
            }
            ItemRef::Wire(id) => {
                let record = self.wire(id)?;
                self.hir
                    .wire(record.wire)?
                    .gate
                    .map(|gate| (gate, record.clause_scope))
            }
            ItemRef::Use(_) => None,
        };
        if let Some((expr, scope)) = gate {
            positions.push(self.gate_position(expr, scope)?);
        }
        Ok(())
    }

    fn gate_position(&mut self, expr: ExprId, scope: ScopeId) -> Result<Position, Internal> {
        #[cfg(test)]
        {
            self.typing.gate_positions += 1;
        }
        let found = self.demand_val(Query::Expr(expr, scope))?;
        let val = self.as_converted(found);
        Ok(Position {
            val,
            ty: Ty::BOOL,
            unit: None,
            role: Role::Gate,
            name: None,
            loc: self.hir.expr(expr)?.loc,
            scope,
            expr,
        })
    }

    pub(super) fn detached_roots(
        &mut self,
        (root, scope): (ExprId, ScopeId),
        (expected, flows): (Option<Scalar>, bool),
        origin: Option<&Position>,
        found: &mut Vec<Detached>,
    ) -> Result<(), Internal> {
        if !matches!(self.memo.exprs.get(&(root, scope)), Some(Val::Error)) {
            return Ok(());
        }
        let mut stack = vec![(root, scope, (expected, flows), false, origin.cloned())];
        while let Some((expr, scope, typed, passed_over, origin)) = stack.pop() {
            if !self.typing.detached_walks.insert((expr, scope)) {
                continue;
            }
            let kind = self.hir.expr(expr)?.kind.clone();
            if let ExprKind::Call(..) = kind
                && let Some(call_scope) = self.memo.call_scopes.get(&(expr, scope)).copied()
                && let ScopeKind::Locals(locals) = &self.scope_of(call_scope)?.kind
                && let LocalsOwner::Fn(function) = locals.owner
            {
                let body = self.hir.fn_decl(function)?.value;
                if matches!(self.memo.exprs.get(&(body, call_scope)), Some(Val::Error)) {
                    stack.push((body, call_scope, typed, passed_over, origin.clone()));
                }
            }
            let children = kind.children();
            let placeheld = children
                .iter()
                .any(|child| matches!(self.memo.exprs.get(&(*child, scope)), Some(Val::Error)));
            if !placeheld {
                continue;
            }
            let chosen = self.chosen_child(&kind, scope)?;
            for (position, child) in children.iter().enumerate() {
                let Some(val) = self.memo.exprs.get(&(*child, scope)).cloned() else {
                    continue;
                };
                let (child_typed, through) = child_flow(&kind, position, typed);
                let origin = origin.clone().filter(|_| through);
                let passed_over = passed_over
                    || chosen
                        .is_some_and(|chosen| position != chosen && arm_position(&kind, position));
                match val {
                    Val::Error => stack.push((*child, scope, child_typed, passed_over, origin)),
                    Val::Scalar(node) => {
                        self.demand_sig(node)?;
                        if self.arrays.contains(&node) {
                            found.push((node, (None, false), passed_over, None));
                        } else {
                            found.push((node, child_typed, passed_over, origin));
                        }
                    }
                    Val::Array(array) => {
                        for node in array.elements.iter().copied() {
                            self.demand_sig(node)?;
                            found.push((node, (None, false), passed_over, None));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn chosen_child(&mut self, kind: &ExprKind, scope: ScopeId) -> Result<Option<usize>, Internal> {
        let (selector, condition) = match kind {
            ExprKind::If(selector, ..) => (*selector, true),
            ExprKind::Match { scrutinee, .. } => (*scrutinee, false),
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Name(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Convert(..)
            | ExprKind::Array(_)
            | ExprKind::Field(..)
            | ExprKind::Index(..)
            | ExprKind::IndexFromEnd(..)
            | ExprKind::Slice { .. }
            | ExprKind::History { .. }
            | ExprKind::Call(..)
            | ExprKind::Unary(..)
            | ExprKind::Binary(..) => return Ok(None),
        };
        let Some(Val::Scalar(node)) = self.memo.exprs.get(&(selector, scope)).cloned() else {
            return Ok(None);
        };
        let sig = self.demand_sig(node)?;
        if sig.reactive || sig.frame || sig.is_error() {
            return Ok(None);
        }
        if !condition {
            return Ok(Some(0));
        }
        Ok(match self.demand_cval(node, EvalTy::Bool)? {
            CVal::Bool(true) => Some(1),
            CVal::Bool(false) => Some(2),
            _ => None,
        })
    }

    fn as_converted(&self, val: Val) -> Val {
        match val {
            Val::Scalar(node) => {
                Val::Scalar(self.memo.string_bytes.get(&node).copied().unwrap_or(node))
            }
            other => other,
        }
    }
}

pub(super) type Detached = (NodeId, (Option<Scalar>, bool), bool, Option<Position>);

fn arm_position(kind: &ExprKind, position: usize) -> bool {
    match kind {
        ExprKind::If(..) => position > 0,
        ExprKind::Match { arms, .. } => {
            position > 0 && (position.is_multiple_of(2) || position > 2 * arms.len())
        }
        ExprKind::Error
        | ExprKind::Number(..)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Name(_)
        | ExprKind::Variant { .. }
        | ExprKind::To
        | ExprKind::Convert(..)
        | ExprKind::Array(_)
        | ExprKind::Field(..)
        | ExprKind::Index(..)
        | ExprKind::IndexFromEnd(..)
        | ExprKind::Slice { .. }
        | ExprKind::History { .. }
        | ExprKind::Call(..)
        | ExprKind::Unary(..)
        | ExprKind::Binary(..) => false,
    }
}

fn child_flow(
    kind: &ExprKind,
    position: usize,
    typed: (Option<Scalar>, bool),
) -> ((Option<Scalar>, bool), bool) {
    let through = (typed, true);
    let own = ((None, false), false);
    let condition = ((Some(Scalar::Bool), true), false);
    match kind {
        ExprKind::Unary(UnaryOp::Not, _) | ExprKind::Binary(BinaryOp::And | BinaryOp::Or, ..) => {
            condition
        }
        ExprKind::Binary(
            BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Is
            | BinaryOp::IsNot,
            ..,
        ) => own,
        ExprKind::Binary(BinaryOp::IntDiv, ..) if typed.0.is_some_and(is_float) => own,
        ExprKind::Unary(UnaryOp::Neg, _) | ExprKind::Binary(..) | ExprKind::Array(_) => through,
        ExprKind::Index(..) | ExprKind::IndexFromEnd(..) | ExprKind::Slice { .. }
            if position == 0 =>
        {
            through
        }
        ExprKind::If(..) if position == 0 => condition,
        ExprKind::If(..) => through,
        ExprKind::Match { .. } if arm_position(kind, position) => through,
        ExprKind::Convert(target, _) if is_float(*target) => ((Some(*target), true), false),
        ExprKind::Error
        | ExprKind::Number(..)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Name(_)
        | ExprKind::Variant { .. }
        | ExprKind::To
        | ExprKind::Convert(..)
        | ExprKind::Field(..)
        | ExprKind::Index(..)
        | ExprKind::IndexFromEnd(..)
        | ExprKind::Slice { .. }
        | ExprKind::History { .. }
        | ExprKind::Call(..)
        | ExprKind::Match { .. } => own,
    }
}
