use std::collections::BTreeMap;

use symphonyscript_core::{AttrUnit, BinaryOp, Scalar, Ty, UnaryOp};

use super::position::Position;
use super::role::Role;
use crate::semantics::Internal;
use crate::semantics::analysis::attr_value_source::AttrValueSource;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::evaluation::own_type;
use crate::semantics::analysis::ids::{InstanceId, NodeId, ScopeId, WireId};
use crate::semantics::analysis::is_float::is_float;
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::reachable::Reachable;
use crate::semantics::analysis::resolve::Found;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Loc, Top};

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
                Top::Fn(_) | Top::Pattern(_) | Top::Placement(_) | Top::Placeholder(_) => {}
            }
        }
        Ok(positions)
    }

    pub(super) fn positions(
        &mut self,
        mut positions: Vec<Position>,
    ) -> Result<Vec<Position>, Internal> {
        for index in 0..self.instances.len() {
            crate::semantics::counted_walk::CountedWalk::PositionsPositions.step();
            let instance = InstanceId(u32::try_from(index).unwrap_or(u32::MAX));
            self.item_positions(ItemRef::Instance(instance), &mut positions)?;
        }
        for index in 0..self.wires.len() {
            crate::semantics::counted_walk::CountedWalk::PositionsPositions.step();
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
                let (params, lets) = {
                    let (params, lets) = self.owner_locals(locals.owner)?;
                    (params.len(), lets.len())
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
        let start = found.len();
        let root_scope = scope;
        let mut faults = Vec::new();
        let mut stack = vec![(
            root,
            scope,
            (expected, flows),
            false,
            origin.cloned(),
            ((root, scope), None, None),
            None,
            false,
        )];
        while let Some((expr, scope, typed, passed_over, origin, flow, picked, quiet)) = stack.pop()
        {
            crate::semantics::counted_walk::CountedWalk::PositionsDetachedRoots.step();
            if let Some((start, _)) = self.memo.eager_starts.get(&(expr, scope)).copied() {
                if passed_over {
                    self.memo.eager_starts.insert((expr, scope), (start, true));
                } else {
                    self.memo.eager_starts.remove(&(expr, scope));
                    self.draw_reached(Reachable::Eval(start, EvalTy::Node));
                }
            }
            if !self.typing.detached_walks.insert((expr, scope)) {
                continue;
            }
            let kind = self.hir.expr(expr)?.kind.clone();
            let loc = self.hir.expr(expr)?.loc;
            let (group, rank, landing) = match (flow, &kind) {
                ((group, rank, None), kind) if !matches!(kind, ExprKind::Call(..)) => {
                    (group, rank, Some((expr, scope)))
                }
                (flow, _) => flow,
            };
            if let ExprKind::Call(..) = kind
                && let Some(call_scope) = self.memo.call_scopes.get(&(expr, scope)).copied()
                && let ScopeKind::Locals(locals) = &self.scope_of(call_scope)?.kind
                && let LocalsOwner::Fn(function) = locals.owner
            {
                let body = self.hir.fn_decl(function)?.value;
                if matches!(self.memo.exprs.get(&(body, call_scope)), Some(Val::Error)) {
                    stack.push((
                        body,
                        call_scope,
                        typed,
                        passed_over,
                        origin.clone(),
                        (group, rank.or(Some(loc)), landing),
                        picked,
                        quiet,
                    ));
                }
            }
            if let ExprKind::Name(name) = &kind
                && let Found::Query(Query::Let(owner, local @ LetRef::Local(_))) =
                    self.find(name, scope)?
                && let Some(decl) = self.let_decl(owner, local)?
            {
                let within = self.let_scope(owner, local);
                let value = decl.value;
                if matches!(self.memo.exprs.get(&(value, within)), Some(Val::Error)) {
                    stack.push((
                        value,
                        within,
                        (None, false),
                        passed_over,
                        None,
                        ((value, within), None, None),
                        None,
                        quiet,
                    ));
                }
            }
            let children = kind.children();
            let refused = !children
                .iter()
                .any(|child| matches!(self.memo.exprs.get(&(*child, scope)), Some(Val::Error)));
            let gapped = self.holds_a_syntax_gap(&kind)?;
            let chosen = self.chosen_child(&kind, scope)?;
            let read = match kind {
                ExprKind::Array(_) => picked,
                _ => None,
            };
            let base_picked = self.picked_element(&kind, scope)?;
            for (position, child) in children.iter().enumerate() {
                let Some(val) = self.memo.exprs.get(&(*child, scope)).cloned() else {
                    continue;
                };
                let unread = read.is_some_and(|read| read != position);
                let (child_typed, through) = match child_flow(&kind, position, typed) {
                    _ if unread || refused => ((None, false), false),
                    flow => flow,
                };
                let origin = origin.clone().filter(|_| through);
                let (child_group, child_rank, child_landing) = if through {
                    (group, rank, landing)
                } else {
                    ((*child, scope), None, None)
                };
                let child_loc = self.hir.expr(*child)?.loc;
                let member = (
                    child_rank.unwrap_or(child_loc),
                    child_landing.unwrap_or((*child, scope)),
                );
                let child_picked = match kind {
                    ExprKind::Index(..) | ExprKind::IndexFromEnd(..) if position == 0 => {
                        base_picked
                    }
                    _ if arm_position(&kind, position) => picked,
                    _ => None,
                };
                let quiet = quiet || gapped;
                let passed_over = passed_over
                    || unread
                    || chosen.is_some_and(|chosen| {
                        chosen != Some(position) && arm_position(&kind, position)
                    });
                match val {
                    Val::Error => stack.push((
                        *child,
                        scope,
                        child_typed,
                        passed_over,
                        origin,
                        (child_group, child_rank, child_landing),
                        child_picked,
                        quiet,
                    )),
                    Val::Scalar(node) => {
                        let sig = self.demand_sig(node)?;
                        if sig.is_error() {
                            faults.push(self.node(node)?.loc);
                        }
                        let (typed, origin) = if self.arrays.contains(&node) {
                            ((None, false), None)
                        } else if sig.kind == Kind::Str && reads_bytes(child_typed, origin.as_ref())
                        {
                            ((None, false), origin)
                        } else {
                            (child_typed, origin)
                        };
                        found.push((node, typed, passed_over, origin, child_group, member, quiet));
                    }
                    Val::Array(array) => {
                        for node in array.elements.iter().copied() {
                            if self.demand_sig(node)?.is_error() {
                                faults.push(self.node(node)?.loc);
                            }
                            found.push((
                                node,
                                (None, false),
                                passed_over,
                                None,
                                child_group,
                                member,
                                quiet,
                            ));
                        }
                    }
                }
            }
        }
        if !faults.is_empty() {
            for entry in found.get_mut(start..).unwrap_or_default() {
                if entry.4 == (root, root_scope) {
                    entry.1 = (None, false);
                    entry.3 = None;
                }
            }
        }
        Ok(())
    }

    pub(super) fn one_fault_per_flow(&mut self, detached: &mut [Detached]) -> Result<(), Internal> {
        let mut groups: BTreeMap<(ExprId, ScopeId), Vec<Ranked>> = BTreeMap::new();
        for (index, (node, .., group, (rank, _), _)) in detached.iter().enumerate() {
            groups
                .entry(*group)
                .or_default()
                .push((*rank, self.node(*node)?.loc, index));
        }
        for mut members in groups.into_values() {
            members.sort();
            let mut refused_kind = None;
            let mut refused_unit = None;
            for (_, _, index) in &members {
                let (node, (expected, flows), _, origin, .., quiet) = detached
                    .get(*index)
                    .cloned()
                    .ok_or(Internal::missed("a detached entry"))?;
                if refused_kind.is_none()
                    && !quiet
                    && flows
                    && let Some(expected) = expected
                    && self.refuses(node, expected)?
                {
                    refused_kind = Some(*index);
                }
                if refused_unit.is_none()
                    && let Some(origin) = origin
                    && origin.role == Role::Attribute
                    && self.refuses_unit(node, origin.unit)?
                {
                    refused_unit = Some(*index);
                }
            }
            if let Some(refused) = refused_kind {
                self.land_outside_bodies(detached, refused)?;
            }
            for (_, _, index) in &members {
                let entry = detached
                    .get_mut(*index)
                    .ok_or(Internal::missed("a detached entry"))?;
                if refused_kind.is_some_and(|refused| refused != *index) {
                    entry.1 = (None, false);
                }
                if refused_unit.is_some_and(|refused| refused != *index) {
                    entry.3 = None;
                }
            }
        }
        Ok(())
    }

    fn land_outside_bodies(
        &mut self,
        detached: &mut [Detached],
        refused: usize,
    ) -> Result<(), Internal> {
        let entry = detached
            .get_mut(refused)
            .ok_or(Internal::missed("a detached entry"))?;
        let (node, (expected, _), (_, (root, scope))) = (entry.0, entry.1, entry.5);
        let landing = self.hir.expr(root)?.loc;
        let Some(expected) = expected else {
            return Ok(());
        };
        if landing.contains(self.node(node)?.loc) {
            return Ok(());
        }
        entry.1 = (None, false);
        let sig = self.known_sig(node)?;
        if let Some((code, message)) = self.incompatibility(node, &sig, expected, false)? {
            self.error(code, landing, scope, message)?;
        }
        Ok(())
    }

    fn refuses_unit(&self, node: NodeId, unit: Option<AttrUnit>) -> Result<bool, Internal> {
        let sig = self.known_sig(node)?;
        if sig.is_error() || !sig.is_numeric() {
            return Ok(false);
        }
        Ok(match (sig.family, unit) {
            (Some(_), None) => true,
            (Some(family), Some(unit)) => family != unit.family,
            (None, _) => false,
        })
    }

    fn picked_element(
        &mut self,
        kind: &ExprKind,
        scope: ScopeId,
    ) -> Result<Option<usize>, Internal> {
        let (base, index, from_end) = match kind {
            ExprKind::Index(base, index) => (*base, *index, false),
            ExprKind::IndexFromEnd(base, index) => (*base, *index, true),
            _ => return Ok(None),
        };
        let ExprKind::Array(elements) = &self.hir.expr(base)?.kind else {
            return Ok(None);
        };
        let length = elements.len();
        let Some(Val::Scalar(node)) = self.memo.exprs.get(&(index, scope)).cloned() else {
            return Ok(None);
        };
        let sig = self.demand_sig(node)?;
        if sig.reactive || sig.frame || sig.is_error() {
            return Ok(None);
        }
        let CVal::Int(value) = self.demand_cval(node, own_type(&sig, EvalTy::Exact))? else {
            return Ok(None);
        };
        let position = if from_end {
            i128::try_from(length).ok().map(|length| length - value)
        } else {
            Some(value)
        };
        Ok(position
            .and_then(|position| usize::try_from(position).ok())
            .filter(|position| *position < length))
    }

    fn holds_a_syntax_gap(&self, kind: &ExprKind) -> Result<bool, Internal> {
        match kind {
            ExprKind::Convert(..)
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
            | ExprKind::For(_) => {
                for child in kind.children() {
                    if matches!(self.hir.expr(child)?.kind, ExprKind::Error) {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Name(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Array(_) => Ok(false),
        }
    }

    fn chosen_child(
        &mut self,
        kind: &ExprKind,
        scope: ScopeId,
    ) -> Result<Option<Option<usize>>, Internal> {
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
            | ExprKind::Stepped(_)
            | ExprKind::Call(..)
            | ExprKind::Unary(..)
            | ExprKind::Binary(..)
            | ExprKind::For(_) => return Ok(None),
        };
        let Some(Val::Scalar(node)) = self.memo.exprs.get(&(selector, scope)).cloned() else {
            return Ok(Some(None));
        };
        let sig = self.demand_sig(node)?;
        if sig.is_error() {
            return Ok(Some(None));
        }
        if sig.reactive || sig.frame {
            return Ok(None);
        }
        if !condition {
            return Ok(Some(Some(0)));
        }
        if !matches!(sig.kind, Kind::Bool | Kind::Any) {
            return Ok(Some(None));
        }
        Ok(Some(match self.demand_cval(node, EvalTy::Bool)? {
            CVal::Bool(true) => Some(1),
            CVal::Bool(false) => Some(2),
            _ => None,
        }))
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

pub(super) type Detached = (
    NodeId,
    (Option<Scalar>, bool),
    bool,
    Option<Position>,
    (ExprId, ScopeId),
    (Loc, (ExprId, ScopeId)),
    bool,
);

type Ranked = (Loc, Loc, usize);

fn reads_bytes(typed: (Option<Scalar>, bool), origin: Option<&Position>) -> bool {
    typed.0 == Some(Scalar::U8)
        && origin.is_some_and(|origin| {
            origin.ty.scalar == Scalar::U8
                && (!origin.ty.lengths.is_empty() || origin.ty.is_variable())
        })
}

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
        | ExprKind::Stepped(_)
        | ExprKind::Call(..)
        | ExprKind::Unary(..)
        | ExprKind::Binary(..)
        | ExprKind::For(_) => false,
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
        | ExprKind::Stepped(_)
        | ExprKind::Call(..)
        | ExprKind::Match { .. }
        | ExprKind::For(_) => own,
    }
}
