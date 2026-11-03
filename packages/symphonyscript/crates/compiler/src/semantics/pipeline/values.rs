use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::leaf::Leaf;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::FloatLit;
use crate::semantics::ir::flat::{Op, binary_of};
use crate::semantics::operators::{
    EvalTy, I64Position, Operands, Operator, Value, outside_i64, plan_op, represents, ty_name,
};
use symphonyscript_core::BinaryOp;

fn class(ty: EvalTy) -> usize {
    match ty {
        EvalTy::F32 | EvalTy::DefaultF32 => 1,
        EvalTy::F64 | EvalTy::DefaultF64 => 2,
        EvalTy::Bool => 3,
        EvalTy::Exact
        | EvalTy::Default
        | EvalTy::U8
        | EvalTy::I32
        | EvalTy::I64
        | EvalTy::Enum(_)
        | EvalTy::Str
        | EvalTy::Node => 0,
    }
}

fn evaluated_in(ty: EvalTy) -> EvalTy {
    match class(ty) {
        1 => EvalTy::F32,
        2 => EvalTy::F64,
        3 => EvalTy::Bool,
        _ => EvalTy::Exact,
    }
}

impl Core<'_> {
    fn typed_nodes(&self, root: u32, ty: EvalTy) -> Result<Vec<(u32, EvalTy)>, Internal> {
        let mut order = Vec::new();
        let mut stack = vec![(root, ty, false)];
        while let Some((node, ty, visited)) = stack.pop() {
            if visited {
                order.push((node, ty));
                continue;
            }
            stack.push((node, ty, true));
            let (op, _, _, _) = self.ir.node(node)?;
            let operands = self.operands(node)?;
            for (position, operand) in operands.into_iter().enumerate().rev() {
                let operand_ty = if op == Op::Index && position == 1 {
                    EvalTy::Exact
                } else {
                    ty
                };
                stack.push((operand, operand_ty, false));
            }
        }
        Ok(order)
    }

    fn memo(&self, at: u32, ty: EvalTy) -> Option<CoreValue> {
        self.let_values
            .get(at as usize)
            .and_then(|slots| slots.get(class(ty)))
            .cloned()
            .flatten()
    }

    pub fn evaluate(&mut self, root: u32, ty: EvalTy) -> Stage<CoreValue> {
        let ty = evaluated_in(ty);
        let mut jobs: Vec<(u32, EvalTy, Option<u32>)> = vec![(root, ty, None)];
        while let Some(&(node, ty, owner)) = jobs.last() {
            let nodes = self.typed_nodes(node, ty)?;
            let mut missing = Vec::new();
            for (read, read_ty) in &nodes {
                if let DeclRef::Let(at) = self.bound(*read)?
                    && !self.cyclic.get(at as usize).copied().unwrap_or(false)
                    && self.memo(at, *read_ty).is_none()
                {
                    missing.push((self.let_root(at)?, evaluated_in(*read_ty), Some(at)));
                }
            }
            if !missing.is_empty() {
                if missing.iter().any(|job| jobs.contains(job)) {
                    return Err(Internal::missed("an open declaration outside every cycle"));
                }
                jobs.extend(missing);
                continue;
            }
            jobs.pop();
            CountedWalk::ValueRun.step();
            let value = match self.run(&nodes)? {
                Ok(value) => value,
                Err(declined) => return Ok(Err(declined)),
            };
            match owner {
                Some(at) => {
                    if let Some(slot) = self
                        .let_values
                        .get_mut(at as usize)
                        .and_then(|slots| slots.get_mut(class(ty)))
                    {
                        *slot = Some(value);
                    }
                }
                None => return Ok(Ok(value)),
            }
        }
        Err(Internal::missed("evaluated root"))
    }

    fn run(&mut self, nodes: &[(u32, EvalTy)]) -> Stage<CoreValue> {
        let mut stack: Vec<CoreValue> = Vec::new();
        let pop = |stack: &mut Vec<CoreValue>| stack.pop().ok_or(Internal::missed("value stack"));
        for (node, ty) in nodes {
            let (node, ty) = (*node, *ty);
            let (op, a, b, _) = self.ir.node(node)?;
            let value = match op {
                Op::Int => {
                    let value = *self
                        .ir
                        .ints
                        .get(a as usize)
                        .ok_or(Internal::missed("IR integer"))?;
                    self.integer_literal(node, value, ty)?
                }
                Op::Float => {
                    let entry = *self
                        .ir
                        .floats
                        .get(a as usize)
                        .ok_or(Internal::missed("IR float"))?;
                    let literal = FloatLit {
                        negative: entry.negative,
                        digits: self.ir.text(entry.digits).to_string(),
                        exponent: entry.exponent,
                    };
                    self.float_literal(node, &literal, ty)?
                }
                Op::Bool => Value::Bool(a == 1),
                Op::Name => match self.bound(node)? {
                    DeclRef::Let(at) => self.memo(at, ty).unwrap_or(Value::Error),
                    DeclRef::Duplicate | DeclRef::None => Value::Error,
                    DeclRef::Instance(_) => return Ok(Err(Declined)),
                },
                Op::Neg => {
                    let operand = pop(&mut stack)?;
                    plan_op(self, Operator::Neg(node, ty), Operands::One(operand))?
                }
                Op::Add | Op::Sub | Op::Mul => {
                    let right = pop(&mut stack)?;
                    let left = pop(&mut stack)?;
                    let binary = binary_of(op).ok_or(Internal::missed("operator"))?;
                    let joins = binary == BinaryOp::Add
                        && matches!((&left, &right), (Value::Array(_), Value::Array(_)));
                    let operator = if joins {
                        Operator::Join(node)
                    } else {
                        Operator::Arith(binary, node, ty)
                    };
                    plan_op(self, operator, Operands::Two(left, right))?
                }
                Op::Array => {
                    let count = b as usize;
                    let start = stack
                        .len()
                        .checked_sub(count)
                        .ok_or(Internal::missed("value stack"))?;
                    let items = stack.split_off(start);
                    plan_op(self, Operator::Array(node), Operands::Many(items))?
                }
                Op::Index => {
                    let position = pop(&mut stack)?;
                    let array = pop(&mut stack)?;
                    plan_op(
                        self,
                        Operator::Element(a, b),
                        Operands::Two(array, position),
                    )?
                }
                Op::Str
                | Op::Variant
                | Op::To
                | Op::Convert
                | Op::Field
                | Op::IndexFromEnd
                | Op::Slice
                | Op::Call
                | Op::NamedCall
                | Op::MethodCall
                | Op::NamedMethodCall
                | Op::Past
                | Op::Stepped
                | Op::Not
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
                | Op::Bad => return Ok(Err(Declined)),
            };
            if self.declined {
                return Ok(Err(Declined));
            }
            stack.push(value);
        }
        Ok(Ok(pop(&mut stack)?))
    }

    fn index_outside_i64(&mut self, index: u32, position: &CoreValue) -> Result<bool, Internal> {
        if let Value::Int(value) = position
            && i64::try_from(*value).is_err()
        {
            self.draw_at(
                Code::IndexOutOfRange,
                index,
                outside_i64(I64Position::Index, *value),
            )?;
            return Ok(true);
        }
        Ok(false)
    }

    fn own_float(&self, root: u32) -> Result<EvalTy, Internal> {
        for node in self.postorder(root)? {
            let (op, a, _, _) = self.ir.node(node)?;
            if op == Op::Float
                && let Some(entry) = self.ir.floats.get(a as usize)
                && entry.value.abs() > f64::from(f32::MAX)
            {
                return Ok(EvalTy::F64);
            }
        }
        Ok(EvalTy::F32)
    }

    pub fn evaluate_own(&mut self, root: u32) -> Stage<()> {
        let mut stack = vec![(root, false)];
        while let Some((node, indexed)) = stack.pop() {
            let fact = self.fact(node)?;
            let own = match fact.leaf {
                Leaf::Int => Some(EvalTy::Exact),
                Leaf::Float => Some(self.own_float(node)?),
                Leaf::Bool => Some(EvalTy::Bool),
                Leaf::Any | Leaf::Error => None,
            };
            let Some(own) = own else {
                let (op, _, _, _) = self.ir.node(node)?;
                for (position, operand) in self.operands(node)?.into_iter().enumerate() {
                    stack.push((operand, op == Op::Index && position == 1));
                }
                continue;
            };
            let value = match self.evaluate(node, own)? {
                Ok(value) => value,
                Err(declined) => return Ok(Err(declined)),
            };
            if indexed && fact.dims.is_empty() {
                self.index_outside_i64(node, &value)?;
            }
        }
        Ok(Ok(()))
    }

    fn integer_literal(
        &mut self,
        node: u32,
        value: i128,
        ty: EvalTy,
    ) -> Result<CoreValue, Internal> {
        match ty {
            EvalTy::F32 | EvalTy::F64 => {
                let literal = FloatLit::from_integer(value);
                let (converted, exactly) = if ty == EvalTy::F32 {
                    let float = literal.to_f32()?;
                    (Value::F32(float), represents(value, f64::from(float)))
                } else {
                    let float = literal.to_f64()?;
                    (Value::F64(float), represents(value, float))
                };
                if !exactly {
                    self.draw_at(
                        Code::DoesNotFit,
                        node,
                        format!("this value does not fit `{}`", ty_name(ty)),
                    )?;
                    return Ok(Value::Error);
                }
                Ok(converted)
            }
            EvalTy::Exact
            | EvalTy::Default
            | EvalTy::U8
            | EvalTy::I32
            | EvalTy::I64
            | EvalTy::DefaultF32
            | EvalTy::DefaultF64
            | EvalTy::Bool
            | EvalTy::Enum(_)
            | EvalTy::Str
            | EvalTy::Node => {
                if value == i128::MIN {
                    self.draw_at(
                        Code::IntegerOverflow,
                        node,
                        "this value's magnitude reaches 2¹²⁷",
                    )?;
                    return Ok(Value::Error);
                }
                Ok(Value::Int(value))
            }
        }
    }

    fn float_literal(
        &mut self,
        node: u32,
        literal: &FloatLit,
        ty: EvalTy,
    ) -> Result<CoreValue, Internal> {
        let value = match ty {
            EvalTy::F32 => {
                let float = literal.to_f32()?;
                float.is_finite().then_some(Value::F32(float))
            }
            EvalTy::F64 => {
                let float = literal.to_f64()?;
                float.is_finite().then_some(Value::F64(float))
            }
            EvalTy::Exact
            | EvalTy::Default
            | EvalTy::U8
            | EvalTy::I32
            | EvalTy::I64
            | EvalTy::DefaultF32
            | EvalTy::DefaultF64
            | EvalTy::Bool
            | EvalTy::Enum(_)
            | EvalTy::Str
            | EvalTy::Node => return Ok(Value::Error),
        };
        match value {
            Some(value) => Ok(value),
            None => {
                self.draw_at(
                    Code::DoesNotFit,
                    node,
                    format!("this value does not fit `{}`", ty_name(ty)),
                )?;
                Ok(Value::Error)
            }
        }
    }
}
