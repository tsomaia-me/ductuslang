use super::core::Core;
use super::fact::Fact;
use super::leaf::Leaf;
use crate::semantics::Internal;
use crate::semantics::ir::Suffix;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::EvalTy;
use symphonyscript_core::Scalar;

impl Core<'_> {
    pub fn literal_suffix(&self, node: u32) -> Result<Suffix, Internal> {
        let (op, _, suffix, _) = self.ir.node(node)?;
        if !matches!(op, Op::Int | Op::Float) || suffix == NONE {
            return Ok(Suffix::None);
        }
        Ok(self
            .ir
            .suffixes
            .get(suffix as usize)
            .copied()
            .unwrap_or(Suffix::None))
    }

    pub fn suffix_ty(&self, node: u32) -> Result<Option<EvalTy>, Internal> {
        Ok(match self.literal_suffix(node)? {
            Suffix::Type(Scalar::U8) => Some(EvalTy::U8),
            Suffix::Type(Scalar::I32) => Some(EvalTy::I32),
            Suffix::Type(Scalar::I64) => Some(EvalTy::I64),
            Suffix::Type(Scalar::F32) => Some(EvalTy::F32),
            Suffix::Type(Scalar::F64) => Some(EvalTy::F64),
            Suffix::Type(_) => None,
            Suffix::None | Suffix::Unit { .. } => None,
        })
    }

    pub fn literal_fact(&self, node: u32, leaf: Leaf) -> Result<Fact, Internal> {
        Ok(match self.suffix_ty(node)? {
            Some(ty) => {
                let leaf = if matches!(ty, EvalTy::F32 | EvalTy::F64) {
                    Leaf::Float
                } else {
                    Leaf::Int
                };
                Fact::fixed(leaf, Some(ty))
            }
            None => match self.literal_suffix(node)? {
                Suffix::Unit { family, unit }
                    if leaf == Leaf::Float || self.unit_scale(family, unit)?.fract() != 0.0 =>
                {
                    Fact::scalar(Leaf::Float)
                }
                Suffix::Unit { .. } | Suffix::None | Suffix::Type(_) => Fact::scalar(leaf),
            },
        })
    }
}

impl Core<'_> {
    pub fn suffixed_integer(
        &mut self,
        node: u32,
        value: i128,
        own: EvalTy,
    ) -> Result<super::core_value::CoreValue, Internal> {
        if let Some(range) = crate::semantics::operators::int_range(own)
            && !range.contains(&value)
        {
            self.draw_at(
                crate::diagnostics::Code::DoesNotFit,
                node,
                format!(
                    "the value {value} does not fit `{}`",
                    crate::semantics::operators::ty_name(own)
                ),
            )?;
            return Ok(crate::semantics::operators::Value::Error);
        }
        self.integer_literal(node, value, own)
    }
}
