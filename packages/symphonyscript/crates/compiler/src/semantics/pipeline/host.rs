use super::core::Core;
use super::core_value::CoreValue;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::operators::{OperatorHost, Value};
use crate::semantics::reporting::grouped;

impl OperatorHost for Core<'_> {
    type Node = u32;
    type Array = u32;
    type Reference = u32;

    fn fault(&mut self, code: Code, node: u32, message: String) -> Result<(), Internal> {
        let calls = self.index_calls(code, node, &message)?;
        let Some(last) = calls.last().copied() else {
            return self.draw_at(code, node, message);
        };
        let origin = self.returned_root(last)?;
        for call in &calls {
            self.push_call(*call);
        }
        let drawn = self.draw_at(code, origin, message);
        for _ in &calls {
            self.eval_chain.pop();
        }
        drawn
    }

    fn fault_helped(
        &mut self,
        code: Code,
        node: u32,
        message: String,
        help: String,
    ) -> Result<(), Internal> {
        let calls = self.index_calls(code, node, &message)?;
        let origin = match calls.last() {
            Some(call) => self.returned_root(*call)?,
            None => node,
        };
        let span = self.node_span(origin)?;
        for call in &calls {
            self.push_call(*call);
        }
        let diagnostic = crate::diagnostics::Diagnostic::new(code, span, message).with_help(help);
        let diagnostic = self.chained(diagnostic);
        self.push_drawn(diagnostic);
        for _ in &calls {
            self.eval_chain.pop();
        }
        Ok(())
    }

    fn within_value_limit(
        &mut self,
        node: u32,
        value: CoreValue,
        built: &str,
    ) -> Result<CoreValue, Internal> {
        let held = match &value {
            Value::Array(_) => self.store.elements(&value),
            Value::Str(text) => u64::try_from(text.chars().count()).unwrap_or(u64::MAX),
            Value::Int(_)
            | Value::F32(_)
            | Value::F64(_)
            | Value::Bool(_)
            | Value::Enum(..)
            | Value::Node(_)
            | Value::Error => return Ok(value),
        };
        let unit = if matches!(value, Value::Str(_)) {
            "characters"
        } else {
            "elements"
        };
        if held > self.limits.value_elements {
            let message = format!(
                "{built} holds {} {unit}, more than the {} a compile-time value holds",
                grouped(held),
                grouped(self.limits.value_elements)
            );
            self.stop_at(Code::ValueLimit, node, message)?;
            return Ok(Value::Error);
        }
        if held > self.limits.value_elements_warning {
            let counted = format!(
                "{built} holds more than {} {unit}",
                grouped(self.limits.value_elements_warning)
            );
            self.warn_at(node, "value_elements", counted, self.limits.value_elements)?;
        }
        Ok(value)
    }

    fn join(&mut self, left: &u32, right: &u32) -> Result<u32, Code> {
        self.store.join(*left, *right)
    }

    fn slice(&mut self, base: &u32, start: usize, end: usize) -> u32 {
        self.store.slice(*base, start, end)
    }

    fn length(&self, array: &u32) -> usize {
        self.store.len(*array) as usize
    }

    fn element(&mut self, array: &u32, position: usize) -> Option<CoreValue> {
        self.store.element(*array, position)
    }

    fn array(&mut self, items: Vec<CoreValue>) -> u32 {
        self.store.literal(items)
    }

    fn lengths(&mut self, value: &CoreValue) -> Vec<u32> {
        self.store.lengths(value)
    }
}

impl Core<'_> {
    fn returning_calls(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let mut calls = Vec::new();
        let mut origin = node;
        while self.ir.node(origin)?.0 == crate::semantics::ir::flat::Op::Call
            && !calls.contains(&origin)
            && let Some(_) = self.callee(origin)?
        {
            calls.push(origin);
            origin = self.returned_root(origin)?;
        }
        Ok(calls)
    }

    fn returned_root(&self, call: u32) -> Result<u32, Internal> {
        let function = self.callee(call)?.ok_or(Internal::missed("a callee"))?;
        self.function_roots(function)?
            .last()
            .copied()
            .ok_or(Internal::missed("a body value"))
    }
}

impl Core<'_> {
    fn index_calls(&self, code: Code, node: u32, message: &str) -> Result<Vec<u32>, Internal> {
        if code == Code::IndexOutOfRange && message.starts_with("the index ") {
            self.returning_calls(node)
        } else {
            Ok(Vec::new())
        }
    }
}
