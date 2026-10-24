mod check;
mod expectations;
mod order;
mod position;
mod positions;
mod synth;
mod typing;
mod units;
mod values;
mod widths;

pub(crate) use typing::Typing;

use crate::diagnostics::Code;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::BindingId;
use crate::semantics::analysis::val::Val;

impl Engine<'_> {
    pub(crate) fn infer(&mut self) {
        let positions = self.positions();
        let mut valid = Vec::new();
        for position in positions {
            let lengths = match &position.val {
                Val::Array(array) => array.shape.lengths(),
                Val::Scalar(_) => Vec::new(),
                Val::Error => continue,
            };
            if lengths != position.ty.lengths {
                self.error(
                    Code::TypeMismatch,
                    position.loc,
                    position.scope,
                    "the value's array shape differs from the attribute's type",
                );
                continue;
            }
            for node in position.val.nodes() {
                self.demand_sig(node);
            }
            valid.push(position);
        }
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            self.demand_sig(self.binding(binding).init);
        }
        self.units(&valid);
        self.read_in_default_units();
        for position in &valid {
            for node in position.val.nodes() {
                self.expect(node, Some(position.ty.scalar));
            }
        }
        for binding in self.binding_order() {
            self.demand_sig(self.binding(binding).init);
            if let Some(own) = self.own_type(binding) {
                self.typing.own.insert(binding, own);
                let init = self.binding(binding).init;
                self.expect(init, Some(own));
            }
        }
        let mut roots = Vec::new();
        for position in &valid {
            for node in position.val.nodes() {
                let condition = position.role == position::Role::Gate;
                self.check(node, Some(position.ty.scalar), condition, &mut roots);
            }
        }
        for index in 0..self.bindings.len() {
            let binding = BindingId(u32::try_from(index).unwrap_or(u32::MAX));
            if let Some(own) = self.typing.own.get(&binding).copied() {
                let init = self.binding(binding).init;
                self.check(init, Some(own), false, &mut roots);
            }
        }
        self.values(&roots);
        self.input_fits();
        self.consistency();
        self.integer_inputs();
    }
}
