use super::answers::ScopedReads;
use super::arg_shape::ArgShape;
use super::c_val::CVal;
use super::engine::Engine;
use super::ids::{NodeId, ScopeId};
use super::limit::Limit;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::{ExprId, FnId, Loc};
use crate::semantics::reporting::grouped;

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn within_value_limit(
        &mut self,
        node: NodeId,
        value: CVal,
        built: &str,
    ) -> Result<CVal, Internal> {
        let (held, unit) = match &value {
            CVal::Array(array) => (array.elements(), "elements"),
            CVal::Str(text) => (
                u64::try_from(text.chars().count()).unwrap_or(u64::MAX),
                "characters",
            ),
            CVal::Int(_)
            | CVal::F32(_)
            | CVal::F64(_)
            | CVal::Bool(_)
            | CVal::Enum(..)
            | CVal::Node(_)
            | CVal::Error => return Ok(value),
        };
        if self.value_boxes.contains(&node) && held <= self.limits.value_elements {
            return Ok(value);
        }
        let at = {
            let record = self.node(node)?;
            (record.loc, record.scope)
        };
        Ok(if self.held_within_value_limit(held, (built, unit), at)? {
            value
        } else {
            CVal::Error
        })
    }

    pub(in crate::semantics::analysis) fn resolved_within_value_limit(
        &mut self,
        expr: ExprId,
        (held, built): (u64, &str),
        at: (Loc, ScopeId),
    ) -> Result<bool, Internal> {
        if held
            <= self
                .limits
                .value_elements_warning
                .min(self.limits.value_elements)
            || self.template_of(at.1)?.is_some()
        {
            return Ok(true);
        }
        if self
            .expr_reads_cell(expr, at.1, &mut ScopedReads::default())?
            .is_cell()
        {
            return Ok(true);
        }
        self.held_within_value_limit(held, (built, "elements"), at)
    }

    fn held_within_value_limit(
        &mut self,
        held: u64,
        (built, unit): (&str, &str),
        (loc, scope): (Loc, ScopeId),
    ) -> Result<bool, Internal> {
        if held > self.limits.value_elements {
            let message = format!(
                "{built} holds {} {unit}, more than the {} a compile-time value holds",
                grouped(held),
                grouped(self.limits.value_elements)
            );
            let help = Code::ValueLimit.help().map(str::to_string);
            self.stop(Code::ValueLimit, loc, Some(scope), (message, help))?;
            return Ok(false);
        }
        if held > self.limits.value_elements_warning && self.value_warnings.insert(loc) {
            let counted = format!(
                "{built} holds more than {} {unit}",
                grouped(self.limits.value_elements_warning)
            );
            self.warn_limit(
                Limit::ValueElements,
                (loc, scope),
                (counted, self.limits.value_elements),
            )?;
        }
        Ok(true)
    }
}

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn arguments_within_value_limit(
        &mut self,
        node: NodeId,
        function: FnId,
        shapes: &[ArgShape],
    ) -> Result<bool, Internal> {
        let mut held = 0u64;
        for shape in shapes {
            if let ArgShape::Array(lengths, _) = shape {
                held = held.max(elements_of(lengths));
            }
        }
        if held
            <= self
                .limits
                .value_elements_warning
                .min(self.limits.value_elements)
        {
            return Ok(true);
        }
        let built = format!(
            "this call of `{}` passes an array that",
            self.function_name(function)?
        );
        let at = {
            let record = self.node(node)?;
            (record.loc, record.scope)
        };
        self.held_within_value_limit(held, (&built, "elements"), at)
    }
}

fn elements_of(lengths: &[u32]) -> u64 {
    let mut elements = 0u64;
    let mut product = 1u64;
    for length in lengths.iter().rev() {
        product = product.saturating_mul(u64::from(*length));
        elements = elements.saturating_add(product);
    }
    elements
}
