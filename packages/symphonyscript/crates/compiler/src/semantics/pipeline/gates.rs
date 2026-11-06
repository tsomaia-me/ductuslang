use super::core::{Core, Stage};
use super::fact::Fact;
use super::leaf::Leaf;
use super::width::target_ty;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op};
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
            if self.kept_non_bool(root)? {
                self.draw_at_root(Code::ConditionType, root, "a gate or a guard is a `bool`")?;
            }
            return self.detached(root, "", EvalTy::Bool);
        }
        if fact.leaf != Leaf::Bool {
            self.draw_at_root(Code::ConditionType, root, "a gate or a guard is a `bool`")?;
            return self.own_check(root);
        }
        self.bind_value(root, "", EvalTy::Bool)
    }

    fn kept_non_bool(&self, root: u32) -> Result<bool, Internal> {
        let (op, a, b, _) = self.ir.node(root)?;
        if self.holds_a_missing_operand(root)? {
            return Ok(false);
        }
        Ok(match op {
            Op::Convert => target_ty(b).is_some_and(|target| target != EvalTy::Bool),
            Op::Index | Op::IndexFromEnd => {
                let base = self.fact(a)?;
                !base.dims.is_empty()
                    && !matches!(base.leaf, Leaf::Placeholder | Leaf::Error)
                    && (base.leaf != Leaf::Bool || base.dims.len() > 1)
            }
            _ => false,
        })
    }
}
