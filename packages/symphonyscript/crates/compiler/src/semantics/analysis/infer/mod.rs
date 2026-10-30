mod array_checks;
mod bare_scrutinees;
mod capacities;
mod check;
mod compile_time_enums;
mod enum_bindings;
mod expectations;
mod fixedness;
mod history_reads;
mod order;
mod position;
mod positions;
mod role;
mod shapes;
mod synth;
mod typing;
mod units;
mod values;
mod widths;

pub(crate) use typing::Typing;

use array_checks::ArrayPosition;
use position::Position;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::destination::Destination;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{BindingId, WireId};
use crate::semantics::analysis::val::Val;

impl Engine<'_> {
    pub(crate) fn infer(&mut self) -> Result<(), Internal> {
        let declared = self.declaration_positions()?;
        self.settle_states()?;
        let positions = self.positions(declared)?;
        let mut valid = Vec::new();
        let mut variable_positions = Vec::new();
        let mut detached = Vec::new();
        for position in positions {
            if let Val::Error = position.val {
                let ty = Some(position.ty.scalar);
                let root = (position.expr, position.scope);
                self.detached_roots(root, (ty, true), Some(&position), &mut detached)?;
                continue;
            }
            if let Val::Scalar(node) = position.val
                && self.refused_cell(node)?
            {
                continue;
            }
            match self.array_position(&position)? {
                ArrayPosition::Usual => {}
                ArrayPosition::Faulty => {
                    self.memo.own_typed.extend(position.val.nodes());
                    continue;
                }
                ArrayPosition::Variable => {
                    for node in position.val.nodes() {
                        self.demand_sig(node)?;
                    }
                    variable_positions.push(position.clone());
                    valid.push(position);
                    continue;
                }
            }
            let lengths = match &position.val {
                Val::Array(array) => array.shape.lengths(),
                Val::Scalar(node) if self.arrays.contains(node) => {
                    if let Some(lengths) = self.fixed_lengths(*node)? {
                        lengths
                    } else {
                        let (code, message) = if position.ty.lengths.is_empty() {
                            self.single_value_fault(&position, "an array")?
                        } else {
                            (
                                Code::TypeMismatch,
                                "the value's array shape differs from the attribute's type".into(),
                            )
                        };
                        self.require_shape(*node, position.ty.lengths.clone(), code, message);
                        position.ty.lengths.clone()
                    }
                }
                Val::Scalar(node) if self.demand_sig(*node)?.is_error() => {
                    position.ty.lengths.clone()
                }
                Val::Scalar(_) => Vec::new(),
                Val::Error => continue,
            };
            if lengths != position.ty.lengths {
                let mut faulty = false;
                for node in position.val.nodes() {
                    faulty |= self.demand_sig(node)?.is_error();
                }
                if !faulty {
                    let (code, message) = if position.ty.lengths.is_empty() {
                        self.single_value_fault(&position, "an array")?
                    } else {
                        (
                            Code::TypeMismatch,
                            "the value's array shape differs from the attribute's type".into(),
                        )
                    };
                    self.error(code, position.loc, position.scope, message)?;
                }
                if position.ty.lengths.is_empty() {
                    self.memo.own_typed.extend(position.val.nodes());
                }
                continue;
            }
            for node in position.val.nodes() {
                self.demand_sig(node)?;
            }
            valid.push(position);
        }
        self.unused_lets()?;
        for root in self.value_roots()? {
            self.detached_roots(root, (None, false), None, &mut detached)?;
        }
        for index in 0..self.bindings.len() {
            crate::semantics::counted_walk::CountedWalk::ModInfer.step();
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            self.demand_sig(self.binding(binding)?.init)?;
        }
        for group in self.memo.group_list.clone() {
            self.demand_sig(group)?;
        }
        self.settle_provisional_kinds()?;
        self.mark_own_history()?;
        let mut unit_positions = valid.clone();
        self.one_fault_per_flow(&mut detached)?;
        for (node, _, _, origin, ..) in &detached {
            if let Some(origin) = origin {
                unit_positions.push(Position {
                    val: Val::Scalar(*node),
                    ..origin.clone()
                });
            }
        }
        self.units(&unit_positions)?;
        self.read_in_default_units()?;
        for position in &valid {
            for node in position.val.nodes() {
                self.expect(node, Some(position.ty.scalar))?;
            }
        }
        for (node, (ty, _), ..) in detached.clone() {
            if ty.is_some() {
                self.expect(node, ty)?;
            }
        }
        for binding in self.binding_order()? {
            self.demand_sig(self.binding(binding)?.init)?;
            if let Some(own) = self.own_type(binding)? {
                self.typing.own.insert(binding, own);
                let init = self.binding(binding)?.init;
                self.expect(init, Some(own))?;
            }
        }
        self.enum_bindings(&valid)?;
        self.bare_scrutinees()?;
        for position in self.carried_input_positions()? {
            for node in position.val.nodes() {
                self.expect(node, Some(position.ty.scalar))?;
            }
            valid.push(position);
        }
        let mut roots = Vec::new();
        for position in &valid {
            for node in position.val.nodes() {
                let condition = position.role == role::Role::Gate;
                self.check(
                    node,
                    (Some(position.ty.scalar), true),
                    condition,
                    &mut roots,
                )?;
            }
        }
        for (node, typed, passed_over, origin, .., quiet) in detached {
            let unit_read = origin.is_some_and(|origin| origin.unit.is_some());
            let typed = if quiet && !unit_read {
                (None, false)
            } else {
                typed
            };
            self.check_detached(node, typed, passed_over, &mut roots)?;
        }
        for node in self.memo.own_typed.clone() {
            self.demand_sig(node)?;
            self.check_detached(node, (None, false), false, &mut roots)?;
        }
        for index in 0..self.bindings.len() {
            crate::semantics::counted_walk::CountedWalk::ModInfer.step();
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            if let Some(own) = self.typing.own.get(&binding).copied() {
                let init = self.binding(binding)?.init;
                let flows = !self.typing.default_widths.contains(&binding);
                self.check(init, (Some(own), flows), false, &mut roots)?;
            }
        }
        for index in 0..self.wires.len() {
            crate::semantics::counted_walk::CountedWalk::ModInfer.step();
            let wire = WireId(u32::try_from(index).unwrap_or(u32::MAX));
            if let Some(Destination::Select { node, .. }) = self.memo.destinations.get(&wire) {
                self.destination_labels(*node)?;
            }
        }
        self.values(&roots)?;
        self.retract_unread_evaluations()?;
        self.capacities()?;
        self.capacity_checks(&variable_positions)?;
        self.native_capacity_checks()?;
        self.state_checks()?;
        self.annotated_capacities()?;
        self.reactive_shapes()?;
        self.input_fits()?;
        self.consistency()?;
        self.consistent_shapes()?;
        self.integer_inputs()?;
        Ok(())
    }
}
