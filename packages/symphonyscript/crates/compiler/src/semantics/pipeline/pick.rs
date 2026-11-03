use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::reason::Reason;
use crate::semantics::Internal;
use crate::semantics::ir::flat::{NONE, Op};
use crate::semantics::operators::{I64Position, Value, index_outside, outside_i64};

impl Core<'_> {
    pub fn view(&self, base: u32) -> Result<Option<(u32, Vec<u32>)>, Internal> {
        let mut node = base;
        let mut slices = Vec::new();
        loop {
            let (op, a, _, _) = self.ir.node(node)?;
            match op {
                Op::Array => return Ok(Some((node, slices))),
                Op::Slice => {
                    slices.push(node);
                    node = a;
                }
                Op::Name => match self.bound(node)? {
                    DeclRef::Let(at) if !self.cyclic.get(at as usize).copied().unwrap_or(false) => {
                        node = self.let_root(at)?;
                    }
                    DeclRef::Let(_)
                    | DeclRef::Cell(_)
                    | DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate => {
                        return Ok(None);
                    }
                },
                Op::Int
                | Op::Float
                | Op::Bool
                | Op::Str
                | Op::Variant
                | Op::To
                | Op::Convert
                | Op::Field
                | Op::Index
                | Op::IndexFromEnd
                | Op::Call
                | Op::NamedCall
                | Op::MethodCall
                | Op::NamedMethodCall
                | Op::Past
                | Op::Stepped
                | Op::Neg
                | Op::Not
                | Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Div
                | Op::IntDiv
                | Op::Rem
                | Op::Pow
                | Op::And
                | Op::Or
                | Op::Lt
                | Op::Le
                | Op::Gt
                | Op::Ge
                | Op::Is
                | Op::IsNot
                | Op::If
                | Op::Match
                | Op::For
                | Op::TyScalar
                | Op::TyName
                | Op::TyArray
                | Op::TyVar
                | Op::Bad => return Ok(None),
            }
        }
    }

    pub fn bounds(&self, slice: u32) -> Result<[u32; 2], Internal> {
        let (_, _, start, _) = self.ir.node(slice)?;
        Ok([self.kid(start)?, self.kid(start + 1)?])
    }

    pub fn pick_operands(&self, read: u32) -> Result<Option<Vec<u32>>, Internal> {
        let (_, a, b, _) = self.ir.node(read)?;
        let Some((_, slices)) = self.view(a)? else {
            return Ok(None);
        };
        let mut operands = Vec::new();
        for slice in slices.iter().rev() {
            for bound in self.bounds(*slice)? {
                if bound != NONE {
                    operands.push(bound);
                }
            }
        }
        operands.push(b);
        Ok(Some(operands))
    }

    pub fn pick(&mut self, read: u32, values: &mut Vec<CoreValue>) -> Stage<Option<u32>> {
        let pop = |values: &mut Vec<CoreValue>| values.pop().ok_or(Internal::missed("value stack"));
        let (op, a, b, _) = self.ir.node(read)?;
        let Some((array, slices)) = self.view(a)? else {
            return Err(Internal::missed("a literal array"));
        };
        let held = self
            .fact(array)?
            .dims
            .iter()
            .fold(1u64, |held, length| held.saturating_mul(u64::from(*length)));
        if held
            > self
                .limits
                .value_elements_warning
                .min(self.limits.value_elements)
        {
            return Ok(Err(Declined(Reason::ValueLimitWarning)));
        }
        let offset = pop(values)?;
        let mut sides = Vec::new();
        for slice in &slices {
            let [lower, upper] = self.bounds(*slice)?;
            let high = if upper == NONE {
                None
            } else {
                Some((upper, pop(values)?))
            };
            let low = if lower == NONE {
                None
            } else {
                Some((lower, pop(values)?))
            };
            let (_, _, _, flags) = self.ir.node(*slice)?;
            sides.push((flags, [low, high]));
        }
        let (_, items, count, _) = self.ir.node(array)?;
        let mut start = 0i128;
        let mut length = i128::from(count);
        for (flags, ends) in sides.into_iter().rev() {
            let mut window = [0, length];
            for (side, end) in ends.into_iter().enumerate() {
                let Some((bound, value)) = end else {
                    continue;
                };
                let Value::Int(value) = value else {
                    return Ok(Ok(None));
                };
                if i64::try_from(value).is_err() {
                    self.draw_at(
                        crate::diagnostics::Code::IndexOutOfRange,
                        bound,
                        outside_i64(I64Position::SliceBound, value),
                    )?;
                    continue;
                }
                let position = if flags & (1 << side) != 0 {
                    length - value
                } else {
                    value
                };
                if let Some(slot) = window.get_mut(side) {
                    *slot = position.clamp(0, length);
                }
            }
            let [low, high] = window;
            start += low;
            length = high.max(low) - low;
        }
        let Value::Int(offset) = offset else {
            return Ok(Ok(None));
        };
        let (position, at) = if op == Op::IndexFromEnd {
            if i64::try_from(offset).is_err() {
                self.draw_at(
                    crate::diagnostics::Code::IndexOutOfRange,
                    b,
                    outside_i64(I64Position::Offset, offset),
                )?;
                return Ok(Ok(None));
            }
            (length - offset, read)
        } else {
            (offset, b)
        };
        if (0..length).contains(&position)
            && let Ok(element) = u32::try_from(start + position)
        {
            return Ok(Ok(Some(self.kid(items + element)?)));
        }
        index_outside(self, at, position, u64::try_from(length).unwrap_or(0))?;
        Ok(Ok(None))
    }
}
