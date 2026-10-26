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
        for position in positions {
            match self.array_position(&position)? {
                ArrayPosition::Usual => {}
                ArrayPosition::Faulty => continue,
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
                Val::Scalar(_) => Vec::new(),
                Val::Error => continue,
            };
            if lengths != position.ty.lengths {
                let (code, message) = if position.ty.lengths.is_empty() {
                    self.single_value_fault(&position, "an array")?
                } else {
                    (
                        Code::TypeMismatch,
                        "the value's array shape differs from the attribute's type".into(),
                    )
                };
                self.error(code, position.loc, position.scope, message)?;
                continue;
            }
            for node in position.val.nodes() {
                self.demand_sig(node)?;
            }
            valid.push(position);
        }
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            self.demand_sig(self.binding(binding)?.init)?;
        }
        for group in self.memo.group_list.clone() {
            self.demand_sig(group)?;
        }
        self.mark_own_history()?;
        self.units(&valid)?;
        self.read_in_default_units()?;
        for position in &valid {
            for node in position.val.nodes() {
                self.expect(node, Some(position.ty.scalar))?;
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
                self.check(node, Some(position.ty.scalar), condition, &mut roots)?;
            }
        }
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            if let Some(own) = self.typing.own.get(&binding).copied() {
                let init = self.binding(binding)?.init;
                self.check(init, Some(own), false, &mut roots)?;
            }
        }
        for index in 0..self.wires.len() {
            let wire = WireId(u32::try_from(index).unwrap_or(u32::MAX));
            if let Some(Destination::Select { node, .. }) = self.memo.destinations.get(&wire) {
                self.destination_labels(*node)?;
            }
        }
        self.values(&roots)?;
        self.capacities()?;
        self.capacity_checks(&variable_positions)?;
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
