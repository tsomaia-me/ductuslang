use super::core::{Core, Stage};
use super::fact::Fact;
use super::leaf::Leaf;
use crate::diagnostics::Code;
use crate::semantics::ir::flat::NONE;
use crate::semantics::operators::EvalTy;

impl Core<'_> {
    pub fn bind_gate(&mut self, root: u32) -> Stage<()> {
        if root == NONE {
            return Ok(Ok(()));
        }
        let fact: Fact = match self.ensure_typed(root)? {
            Ok(fact) => fact,
            Err(declined) => return Ok(Err(declined)),
        };
        if !fact.dims.is_empty() {
            let value = if self.variable_length(root)? {
                "a variable array"
            } else {
                "an array"
            };
            self.draw_at(
                Code::ConditionType,
                root,
                format!("a gate or a guard is one `bool`, and this value is {value}"),
            )?;
            return self.own_check(root);
        }
        if fact.leaf == Leaf::Error && fact.node && fact.dims.is_empty() {
            self.draw_at_root(Code::ConditionType, root, "a gate or a guard is a `bool`")?;
            return Ok(Ok(()));
        }
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return self.detached(root, "", EvalTy::Bool);
        }
        if fact.leaf != Leaf::Bool {
            self.draw_at_root(Code::ConditionType, root, "a gate or a guard is a `bool`")?;
            return self.own_check(root);
        }
        self.bind_value(root, "", EvalTy::Bool)
    }
}
