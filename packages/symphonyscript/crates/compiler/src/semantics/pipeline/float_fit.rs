use super::core::Core;
use super::core_value::CoreValue;
use super::leaf::Leaf;
use super::step::Step;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::operators::{EvalTy, Value, ty_name};

impl Core<'_> {
    pub fn loose_float(&self, operand: u32) -> Result<bool, Internal> {
        let fact = self.fact(operand)?;
        Ok(fact.own().is_none()
            && fact.dims.is_empty()
            && matches!(fact.leaf, Leaf::Float | Leaf::Int)
            && !self.is_reactive(operand)?
            && !self.is_kept(operand)?)
    }

    pub fn float_choose(
        &mut self,
        operand: u32,
        value: CoreValue,
        steps: &mut Vec<Step>,
        values: &mut Vec<CoreValue>,
    ) -> Result<(), Internal> {
        match value {
            Value::F64(float) if float.is_nan() || (float as f32).is_finite() => {
                self.unflowed.insert(operand);
                steps.push(Step::FloatFit(operand, EvalTy::F32));
                steps.push(Step::Visit(operand, EvalTy::DefaultF32));
            }
            Value::F64(_) => self.float_fit(operand, value, EvalTy::F64, values)?,
            other => values.push(other),
        }
        Ok(())
    }

    pub fn float_fit(
        &mut self,
        operand: u32,
        value: CoreValue,
        ty: EvalTy,
        values: &mut Vec<CoreValue>,
    ) -> Result<(), Internal> {
        let finite = match value {
            Value::F32(float) => float.is_finite(),
            Value::F64(float) => float.is_finite(),
            _ => true,
        };
        if finite {
            values.push(value);
            return Ok(());
        }
        self.draw_at(
            Code::DoesNotFit,
            operand,
            format!("this value does not fit `{}`", ty_name(ty)),
        )?;
        values.push(Value::Error);
        Ok(())
    }
}
