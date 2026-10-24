use symphonyscript_core::Ty;

use super::position::Position;
use super::role::Role;
use crate::semantics::analysis::attr_value_source::AttrValueSource;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{InstanceId, ScopeId, WireId};
use crate::semantics::analysis::item_ref::ItemRef;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, Top};

impl Engine<'_> {
    pub(crate) fn positions(&mut self) -> Vec<Position> {
        let mut positions = Vec::new();
        let hir = self.hir;
        for (position, top) in hir.top.iter().enumerate() {
            if hir.is_prelude_top(position) {
                continue;
            }
            match *top {
                Top::Input(input) => {
                    self.demand(Query::Input(input));
                    if let Some(position) = self.input_position(input) {
                        positions.push(position);
                    }
                }
                Top::Let(id) => {
                    let global = self.source_global;
                    self.demand(Query::Let(global, LetRef::Top(id)));
                }
                Top::Fn(_) | Top::Pattern(_) | Top::Placement(_) => {}
            }
        }
        for index in 0..self.instances.len() {
            let instance = InstanceId(u32::try_from(index).unwrap_or(u32::MAX));
            self.item_positions(ItemRef::Instance(instance), &mut positions);
        }
        for index in 0..self.wires.len() {
            let wire = WireId(u32::try_from(index).unwrap_or(u32::MAX));
            self.demand(Query::Destination(wire));
            self.item_positions(ItemRef::Wire(wire), &mut positions);
        }
        for (expr, scope) in self.gates.clone() {
            positions.push(self.gate_position(expr, scope));
        }
        let mut scope = 0;
        while scope < self.scopes.len() {
            let id = ScopeId(u32::try_from(scope).unwrap_or(u32::MAX));
            if let ScopeKind::Locals(locals) = &self.scope_of(id).kind {
                let (params, lets) = match locals.owner {
                    LocalsOwner::Fn(function) => {
                        let decl = self.hir.fn_decl(function);
                        (decl.params.len(), decl.lets.len())
                    }
                    LocalsOwner::Pattern(pattern) => {
                        let decl = self.hir.pattern(pattern);
                        (decl.params.len(), decl.lets.len())
                    }
                };
                for index in 0..params {
                    self.demand(Query::Param(id, u32::try_from(index).unwrap_or(u32::MAX)));
                }
                for index in 0..lets {
                    self.demand(Query::Let(
                        id,
                        LetRef::Local(u32::try_from(index).unwrap_or(u32::MAX)),
                    ));
                }
            }
            scope += 1;
        }
        positions
    }

    fn input_position(&mut self, input: crate::semantics::ir::InputId) -> Option<Position> {
        let decl = self.hir.input(input);
        let global = self.source_global;
        let val = self.demand_val(Query::Expr(decl.value, global));
        let bound = self.memo.inputs.get(&input).cloned()?;
        let first = match &bound {
            BoundVal::Scalar(crate::semantics::analysis::bound::Bound::Cell(binding)) => *binding,
            BoundVal::Array(array) => match array.1.first() {
                Some(crate::semantics::analysis::bound::Bound::Cell(binding)) => *binding,
                Some(crate::semantics::analysis::bound::Bound::Value(_)) | None => return None,
            },
            BoundVal::Scalar(crate::semantics::analysis::bound::Bound::Value(_))
            | BoundVal::Error => return None,
        };
        let scalar = self.input_type(first)?;
        let mut ty = Ty::of(scalar);
        if let Val::Array(array) = &val {
            ty.lengths = array.shape.lengths();
        }
        Some(Position {
            val,
            ty,
            unit: None,
            role: Role::Initializer,
            loc: self.hir.expr(decl.value).loc,
            scope: global,
        })
    }

    pub(crate) fn input_type(
        &mut self,
        binding: crate::semantics::analysis::ids::BindingId,
    ) -> Option<symphonyscript_core::Scalar> {
        let record = self.binding(binding).clone();
        if let Some(annotation) = record.annotation {
            return Some(annotation);
        }
        let sig = self.demand_sig(record.init);
        match sig.kind {
            Kind::Int => Some(symphonyscript_core::Scalar::I64),
            Kind::Float => Some(symphonyscript_core::Scalar::F64),
            Kind::Bool => Some(symphonyscript_core::Scalar::Bool),
            Kind::Enum(id) => Some(symphonyscript_core::Scalar::Enum(id)),
            Kind::Str | Kind::Node | Kind::Bare | Kind::Any | Kind::Error => None,
        }
    }

    fn item_positions(&mut self, item: ItemRef, positions: &mut Vec<Position>) {
        let sources = self.attr_sources(item).to_vec();
        for source in sources {
            let Some(decl) = self.attr_decl(item, source.attr).cloned() else {
                continue;
            };
            self.demand(Query::Attr(item, source.attr));
            if let AttrValueSource::Expr(expr, scope) = source.value {
                let val = self.demand_val(Query::Expr(expr, scope));
                positions.push(Position {
                    val,
                    ty: decl.ty.clone(),
                    unit: decl.unit,
                    role: Role::Attribute,
                    loc: self.hir.expr(expr).loc,
                    scope,
                });
            }
        }
        let gate = match item {
            ItemRef::Instance(id) => self.instance(id).and_then(|record| {
                self.hir
                    .placement(record.placement)
                    .gate
                    .map(|gate| (gate, record.scope))
            }),
            ItemRef::Wire(id) => self.wire(id).and_then(|record| {
                self.hir
                    .wire(record.wire)
                    .gate
                    .map(|gate| (gate, record.clause_scope))
            }),
            ItemRef::Use(_) => None,
        };
        if let Some((expr, scope)) = gate {
            positions.push(self.gate_position(expr, scope));
        }
    }

    fn gate_position(&mut self, expr: ExprId, scope: ScopeId) -> Position {
        let val = self.demand_val(Query::Expr(expr, scope));
        Position {
            val,
            ty: Ty::BOOL,
            unit: None,
            role: Role::Gate,
            loc: self.hir.expr(expr).loc,
            scope,
        }
    }
}
