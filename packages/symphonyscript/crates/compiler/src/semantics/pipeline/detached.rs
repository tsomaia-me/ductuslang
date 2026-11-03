use super::core::{Core, Stage};
use super::leaf::Leaf;
use super::width::{target_ty, widens};
use crate::semantics::Internal;
use crate::semantics::ir::flat::Op;
use crate::semantics::operators::{EvalTy, Value, int_range};

struct Detached {
    node: u32,
    ty: Option<EvalTy>,
    group: u32,
    unchecked: bool,
}

const BINARY: [Op; 15] = [
    Op::Add,
    Op::Sub,
    Op::Mul,
    Op::Div,
    Op::IntDiv,
    Op::Rem,
    Op::Pow,
    Op::And,
    Op::Or,
    Op::Lt,
    Op::Le,
    Op::Gt,
    Op::Ge,
    Op::Is,
    Op::IsNot,
];

impl Core<'_> {
    fn faulty(&self, node: u32) -> Result<bool, Internal> {
        Ok(matches!(
            self.fact(node)?.leaf,
            Leaf::Placeholder | Leaf::Error
        ))
    }

    fn unchosen(&mut self, condition: u32) -> Result<bool, Internal> {
        if self.faulty(condition)? {
            return Ok(true);
        }
        if self.is_reactive(condition)? {
            return Ok(false);
        }
        let fact = self.fact(condition)?;
        if fact.leaf != Leaf::Bool
            || !fact.dims.is_empty()
            || self.holds_name_placeholder(condition)?
        {
            return Ok(true);
        }
        Ok(matches!(
            self.evaluate(condition, EvalTy::Bool)?,
            Ok(Value::Error)
        ))
    }

    fn flowed(
        &self,
        node: u32,
        position: usize,
        ty: Option<EvalTy>,
    ) -> Result<(Option<EvalTy>, bool), Internal> {
        let (op, a, b, _) = self.ir.node(node)?;
        let float = matches!(ty, Some(EvalTy::F32 | EvalTy::F64));
        let read = matches!(op, Op::Index | Op::IndexFromEnd | Op::Slice);
        Ok(if matches!(op, Op::Not | Op::And | Op::Or) {
            (Some(EvalTy::Bool), false)
        } else if matches!(op, Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot)
            || (op == Op::IntDiv && float)
        {
            (None, false)
        } else if BINARY.contains(&op) || matches!(op, Op::Neg | Op::Array) {
            (ty, true)
        } else if read && position == 0 {
            if self.fact(a)?.dims.is_empty() {
                (None, false)
            } else {
                (ty, true)
            }
        } else if op == Op::If && position == 0 {
            if self.faulty(a)? || self.fact(node)?.leaf != Leaf::Error {
                (Some(EvalTy::Bool), false)
            } else {
                (None, false)
            }
        } else if op == Op::If {
            (ty, true)
        } else if op == Op::Convert
            && let Some(target @ (EvalTy::F32 | EvalTy::F64)) = target_ty(b)
        {
            (Some(target), false)
        } else {
            (None, false)
        })
    }

    fn picked_position(&mut self, read: u32) -> Result<Option<usize>, Internal> {
        let (op, a, b, _) = self.ir.node(read)?;
        if !matches!(op, Op::Index | Op::IndexFromEnd)
            || self.ir.node(a)?.0 != Op::Array
            || self.faulty(b)?
            || self.is_reactive(b)?
        {
            return Ok(None);
        }
        let count = i128::from(self.ir.node(a)?.2);
        let Ok(Value::Int(offset)) = self.evaluate(b, EvalTy::Exact)? else {
            return Ok(None);
        };
        let position = if op == Op::IndexFromEnd {
            count - offset
        } else {
            offset
        };
        Ok(usize::try_from(position).ok())
    }

    fn refuses(&self, node: u32, ty: EvalTy) -> Result<bool, Internal> {
        let fact = self.fact(node)?;
        if !fact.dims.is_empty() {
            return Ok(false);
        }
        let integer = int_range(ty).is_some();
        Ok(match fact.leaf {
            Leaf::Placeholder | Leaf::Error => false,
            Leaf::Bool => ty != EvalTy::Bool,
            Leaf::Empty | Leaf::Int | Leaf::Float if ty == EvalTy::Bool => true,
            Leaf::Float if integer => true,
            Leaf::Empty | Leaf::Int | Leaf::Float => fact.fixed.is_some_and(|own| !widens(own, ty)),
        })
    }

    pub fn detached(&mut self, root: u32, name: &str, ty: EvalTy) -> Stage<()> {
        if self.fact(root)?.leaf == Leaf::Error && self.is_reactive(root)? {
            return self.evaluate_own(root);
        }
        let mut found: Vec<Detached> = Vec::new();
        let mut faults = false;
        let mut stack = vec![(root, Some(ty), root, None::<usize>, false)];
        while let Some((node, typed, group, picked, unchecked)) = stack.pop() {
            let (op, a, _, _) = self.ir.node(node)?;
            if matches!(op, Op::Call | Op::NamedCall | Op::For) {
                continue;
            }
            let children = self.operands(node)?;
            let mut refused = true;
            let mut beside_fault = false;
            for child in &children {
                if self.faulty(*child)? {
                    refused = false;
                    beside_fault |= BINARY.contains(&op) && self.ir.node(*child)?.0 != Op::Name;
                }
            }
            let base_picked = if self.ir.node(a).is_ok() {
                self.picked_position(node)?
            } else {
                None
            };
            for (position, child) in children.into_iter().enumerate() {
                let unread = (op == Op::Array && picked.is_some_and(|picked| picked != position))
                    || (op == Op::If && position > 0 && self.faulty(a)?);
                let child_unchecked =
                    unchecked || (op == Op::If && position > 0 && self.unchosen(a)?);
                let (child_ty, through) = if refused || beside_fault || unread {
                    (None, false)
                } else {
                    self.flowed(node, position, typed)?
                };
                let child_group = if through { group } else { child };
                let child_picked = if matches!(op, Op::Index | Op::IndexFromEnd) && position == 0 {
                    base_picked
                } else {
                    None
                };
                let reactive_fault =
                    self.fact(child)?.leaf == Leaf::Error && self.is_reactive(child)?;
                if reactive_fault {
                    faults = true;
                }
                if self.faulty(child)? && !reactive_fault {
                    stack.push((child, child_ty, child_group, child_picked, child_unchecked));
                } else {
                    found.push(Detached {
                        node: child,
                        ty: child_ty,
                        group: child_group,
                        unchecked: child_unchecked,
                    });
                }
            }
        }
        if faults {
            for entry in &mut found {
                if entry.group == root {
                    entry.ty = None;
                }
            }
        }
        let mut order: Vec<(u32, u32, usize)> = Vec::new();
        for (index, entry) in found.iter().enumerate() {
            let (lo, _) = self.ir.span(entry.node)?;
            order.push((entry.group, lo, index));
        }
        order.sort_unstable();
        let mut kept = vec![false; found.len()];
        let mut group = None;
        let mut taken = false;
        for (entry_group, _, index) in order {
            if group != Some(entry_group) {
                group = Some(entry_group);
                taken = false;
            }
            let Some(entry) = found.get(index) else {
                continue;
            };
            if taken {
                continue;
            }
            if let Some(ty) = entry.ty
                && self.refuses(entry.node, ty)?
            {
                taken = true;
                if let Some(slot) = kept.get_mut(index) {
                    *slot = true;
                }
            }
        }
        for (index, entry) in found.iter().enumerate() {
            if !kept.get(index).copied().unwrap_or(false) {
                continue;
            }
            let Some(ty) = entry.ty else {
                continue;
            };
            if matches!(self.ir.node(entry.node)?.0, Op::Call | Op::NamedCall) {
                continue;
            }
            let unchecked = self.unchecked || entry.unchecked;
            let outer = std::mem::replace(&mut self.unchecked, unchecked);
            let outcome = self.bind_value(entry.node, name, ty);
            self.unchecked = outer;
            if let Err(declined) = outcome? {
                return Ok(Err(declined));
            }
        }
        self.own_check(root)
    }
}
