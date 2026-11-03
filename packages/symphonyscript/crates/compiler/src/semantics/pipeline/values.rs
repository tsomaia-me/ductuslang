use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::frame::Frame;
use super::leaf::Leaf;
use super::reason::Reason;
use super::step::Step;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::FloatLit;
use crate::semantics::ir::flat::{NONE, Op, binary_of};
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
    fn memo(&self, at: u32, ty: EvalTy) -> Option<CoreValue> {
        self.let_values
            .get(at as usize)
            .and_then(|slots| slots.get(class(ty)))
            .cloned()
            .flatten()
    }

    fn pair_ty(&self, a: u32, b: u32) -> Result<EvalTy, Internal> {
        let (x, y) = (self.fact(a)?.leaf, self.fact(b)?.leaf);
        Ok(match (x, y) {
            (Leaf::Bool, _) | (_, Leaf::Bool) => EvalTy::Bool,
            (Leaf::Float, _) | (_, Leaf::Float) => EvalTy::F64,
            _ => EvalTy::Exact,
        })
    }

    fn visit(&mut self, node: u32, ty: EvalTy, steps: &mut Vec<Step>) -> Result<(), Internal> {
        let (op, a, b, c) = self.ir.node(node)?;
        steps.push(Step::Apply(node, ty));
        let float = matches!(ty, EvalTy::F32 | EvalTy::F64);
        let operands: Vec<(u32, EvalTy)> = match op {
            Op::Neg | Op::Add | Op::Sub | Op::Mul | Op::Rem | Op::Pow => self
                .operands(node)?
                .into_iter()
                .map(|at| (at, ty))
                .collect(),
            Op::Div => {
                let div = if float { ty } else { EvalTy::F32 };
                vec![(a, div), (b, div)]
            }
            Op::IntDiv => vec![(a, EvalTy::Exact), (b, EvalTy::Exact)],
            Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot => {
                let pair = self.pair_ty(a, b)?;
                vec![(a, pair), (b, pair)]
            }
            Op::And | Op::Or => vec![(a, EvalTy::Bool), (b, EvalTy::Bool)],
            Op::Not => vec![(a, EvalTy::Bool)],
            Op::If => {
                steps.pop();
                steps.push(Step::Choose(b, c, ty));
                vec![(a, EvalTy::Bool)]
            }
            Op::Array => self
                .operands(node)?
                .into_iter()
                .map(|at| (at, ty))
                .collect(),
            Op::Index | Op::IndexFromEnd => match self.pick_operands(node)? {
                Some(operands) => {
                    steps.pop();
                    steps.push(Step::Pick(node, ty));
                    operands.into_iter().map(|at| (at, EvalTy::Exact)).collect()
                }
                None => vec![(a, ty), (b, EvalTy::Exact)],
            },
            Op::Slice => {
                let mut operands = vec![(a, ty)];
                for bound in self.bounds(node)? {
                    if bound != NONE {
                        operands.push((bound, EvalTy::Exact));
                    }
                }
                operands
            }
            Op::Int
            | Op::Float
            | Op::Bool
            | Op::Name
            | Op::Str
            | Op::Variant
            | Op::To
            | Op::Convert
            | Op::Field
            | Op::Call
            | Op::NamedCall
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Past
            | Op::Stepped
            | Op::Match
            | Op::For
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => Vec::new(),
        };
        for (operand, operand_ty) in operands.into_iter().rev() {
            steps.push(Step::Visit(operand, operand_ty));
        }
        Ok(())
    }

    pub fn evaluate(&mut self, root: u32, ty: EvalTy) -> Stage<CoreValue> {
        let ty = evaluated_in(ty);
        let mut frames: Vec<Frame> = vec![Frame {
            owner: None,
            ty,
            steps: vec![Step::Visit(root, ty)],
            values: Vec::new(),
        }];
        loop {
            let Some(frame) = frames.last_mut() else {
                return Err(Internal::missed("evaluation frame"));
            };
            let Some(step) = frame.steps.pop() else {
                let value = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                let (owner, frame_ty) = (frame.owner, frame.ty);
                frames.pop();
                CountedWalk::ValueRun.step();
                match owner {
                    Some(at) => {
                        if let Some(slot) = self
                            .let_values
                            .get_mut(at as usize)
                            .and_then(|slots| slots.get_mut(class(frame_ty)))
                        {
                            *slot = Some(value.clone());
                        }
                        if let Some(parent) = frames.last_mut() {
                            parent.values.push(value);
                        }
                    }
                    None => return Ok(Ok(value)),
                }
                continue;
            };
            match step {
                Step::Visit(node, node_ty) => {
                    let (op, _, _, _) = self.ir.node(node)?;
                    if op == Op::Name
                        && let DeclRef::Let(at) = self.bound(node)?
                        && !self.cyclic.get(at as usize).copied().unwrap_or(false)
                    {
                        frame.steps.push(Step::Record(node, false, at, node_ty));
                        if let Some(value) = self.memo(at, node_ty) {
                            frame.values.push(value);
                            continue;
                        }
                        if frames.iter().any(|open| open.owner == Some(at)) {
                            return Err(Internal::missed(
                                "an open declaration outside every cycle",
                            ));
                        }
                        let let_ty = evaluated_in(node_ty);
                        let root = self.let_root(at)?;
                        frames.push(Frame {
                            owner: Some(at),
                            ty: let_ty,
                            steps: vec![Step::Visit(root, let_ty)],
                            values: Vec::new(),
                        });
                        continue;
                    }
                    let mut steps = std::mem::take(&mut frame.steps);
                    self.visit(node, node_ty, &mut steps)?;
                    if let Some(frame) = frames.last_mut() {
                        frame.steps = steps;
                    }
                }
                Step::Choose(then, otherwise, arm_ty) => {
                    let test = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                    match test {
                        Value::Bool(true) => frame.steps.push(Step::Visit(then, arm_ty)),
                        Value::Bool(false) => frame.steps.push(Step::Visit(otherwise, arm_ty)),
                        Value::Int(_)
                        | Value::F32(_)
                        | Value::F64(_)
                        | Value::Enum(..)
                        | Value::Str(_)
                        | Value::Node(_)
                        | Value::Array(_)
                        | Value::Error => frame.values.push(Value::Error),
                    }
                }
                Step::Pick(read, element_ty) => {
                    let mut values = std::mem::take(&mut frame.values);
                    let picked = match self.pick(read, &mut values)? {
                        Ok(picked) => picked,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    let Some(frame) = frames.last_mut() else {
                        return Err(Internal::missed("evaluation frame"));
                    };
                    frame.values = values;
                    match picked {
                        Some(element) => {
                            frame
                                .steps
                                .push(Step::Record(read, true, element, element_ty));
                            frame.steps.push(Step::Visit(element, element_ty));
                        }
                        None => frame.values.push(Value::Error),
                    }
                }
                Step::Record(read, element, target, read_ty) => {
                    if let Some(value) = frame.values.last()
                        && !value.is_error()
                    {
                        let value = value.clone();
                        self.uses
                            .entry((read, class(read_ty)))
                            .or_insert(((element, target), value));
                    }
                }
                Step::Apply(node, node_ty) => {
                    let mut values = std::mem::take(&mut frame.values);
                    let value = match self.apply(node, node_ty, &mut values)? {
                        Ok(value) => value,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    if self.declined {
                        return Ok(Err(Declined(Reason::ValueLimitWarning)));
                    }
                    values.push(value);
                    if let Some(frame) = frames.last_mut() {
                        frame.values = values;
                    }
                }
            }
        }
    }

    fn apply(&mut self, node: u32, ty: EvalTy, stack: &mut Vec<CoreValue>) -> Stage<CoreValue> {
        let pop = |stack: &mut Vec<CoreValue>| stack.pop().ok_or(Internal::missed("value stack"));
        let (op, a, b, _) = self.ir.node(node)?;
        Ok(Ok(match op {
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
                DeclRef::Instance(_) => return Ok(Err(Declined(Reason::NameOfInstance))),
            },
            Op::Neg => {
                let operand = pop(stack)?;
                plan_op(self, Operator::Neg(node, ty), Operands::One(operand))?
            }
            Op::Not => {
                let operand = pop(stack)?;
                plan_op(self, Operator::Not, Operands::One(operand))?
            }
            Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Rem
            | Op::Pow
            | Op::Div
            | Op::IntDiv
            | Op::And
            | Op::Or
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::Is
            | Op::IsNot => {
                let right = pop(stack)?;
                let left = pop(stack)?;
                let binary = binary_of(op).ok_or(Internal::missed("operator"))?;
                let operator = self.binary_operator(node, binary, ty, (&left, &right));
                plan_op(self, operator, Operands::Two(left, right))?
            }
            Op::Array => {
                let start = stack
                    .len()
                    .checked_sub(b as usize)
                    .ok_or(Internal::missed("value stack"))?;
                let items = stack.split_off(start);
                plan_op(self, Operator::Array(node), Operands::Many(items))?
            }
            Op::Index => {
                let position = pop(stack)?;
                let array = pop(stack)?;
                plan_op(
                    self,
                    Operator::Element(a, b),
                    Operands::Two(array, position),
                )?
            }
            Op::IndexFromEnd => {
                let offset = pop(stack)?;
                let array = pop(stack)?;
                if let (Value::Array(held), Value::Int(offset)) = (&array, offset) {
                    if i64::try_from(offset).is_err() {
                        self.draw_at(
                            Code::IndexOutOfRange,
                            b,
                            outside_i64(I64Position::Offset, offset),
                        )?;
                        Value::Error
                    } else {
                        let length = i128::from(self.store.len(*held));
                        plan_op(
                            self,
                            Operator::Element(a, node),
                            Operands::Two(array, Value::Int(length - offset)),
                        )?
                    }
                } else {
                    Value::Error
                }
            }
            Op::Slice => {
                let [lower, upper] = self.bounds(node)?;
                let (_, _, _, flags) = self.ir.node(node)?;
                let present = usize::from(lower != NONE) + usize::from(upper != NONE);
                let start = stack
                    .len()
                    .checked_sub(present + 1)
                    .ok_or(Internal::missed("value stack"))?;
                let values = stack.split_off(start);
                let lower = (lower != NONE).then_some((lower, flags & 1 != 0));
                let upper = (upper != NONE).then_some((upper, flags & 2 != 0));
                plan_op(
                    self,
                    Operator::Slice(a, lower, upper),
                    Operands::Many(values),
                )?
            }
            Op::If
            | Op::Str
            | Op::Variant
            | Op::To
            | Op::Convert
            | Op::Field
            | Op::Call
            | Op::NamedCall
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Past
            | Op::Stepped
            | Op::Match
            | Op::For
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => return Err(Internal::missed("an op the coverage table leaves out")),
        }))
    }

    fn binary_operator(
        &self,
        node: u32,
        binary: BinaryOp,
        ty: EvalTy,
        (left, right): (&CoreValue, &CoreValue),
    ) -> Operator<u32> {
        let float = matches!(ty, EvalTy::F32 | EvalTy::F64);
        match binary {
            BinaryOp::Add if matches!((left, right), (Value::Array(_), Value::Array(_))) => {
                Operator::Join(node)
            }
            BinaryOp::Div => Operator::Div,
            BinaryOp::IntDiv if float => Operator::IntDivFloat(node, ty, true),
            BinaryOp::And | BinaryOp::Or => Operator::Logic(binary),
            BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Is
            | BinaryOp::IsNot => Operator::Compare(binary),
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::IntDiv
            | BinaryOp::Rem
            | BinaryOp::Pow => Operator::Arith(binary, node, ty),
        }
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

    fn own_ty(&self, node: u32) -> Result<Option<EvalTy>, Internal> {
        let fact = self.fact(node)?;
        Ok(match fact.leaf {
            Leaf::Empty | Leaf::Int => Some(EvalTy::Exact),
            Leaf::Float => Some(self.own_float(node)?),
            Leaf::Bool => Some(EvalTy::Bool),
            Leaf::Placeholder | Leaf::Error => None,
        })
    }

    fn known(
        &mut self,
        node: u32,
        ty: EvalTy,
        stack: &mut Vec<(u32, bool)>,
    ) -> Stage<Option<CoreValue>> {
        if self.own_ty(node)?.is_none() || self.is_kept(node)? {
            stack.push((node, false));
            return Ok(Ok(None));
        }
        let value = match self.evaluate(node, ty)? {
            Ok(value) => value,
            Err(declined) => return Ok(Err(declined)),
        };
        Ok(Ok(Some(value)))
    }

    fn quiet_pick(&self, read: u32) -> Result<bool, Internal> {
        let (_, a, _, _) = self.ir.node(read)?;
        Ok(match self.view(a)? {
            Some((array, _)) => matches!(self.fact(array)?.leaf, Leaf::Placeholder | Leaf::Error),
            None => false,
        })
    }

    pub fn evaluate_own(&mut self, root: u32) -> Stage<()> {
        let mut stack = vec![(root, false)];
        while let Some((node, indexed)) = stack.pop() {
            let fact = self.fact(node)?;
            let own = self.own_ty(node)?;
            if let Some(own) = own
                && !self.is_kept(node)?
            {
                let value = match self.evaluate(node, own)? {
                    Ok(value) => value,
                    Err(declined) => return Ok(Err(declined)),
                };
                if indexed && fact.dims.is_empty() {
                    self.index_outside_i64(node, &value)?;
                }
                continue;
            }
            let (op, a, b, c) = self.ir.node(node)?;
            match op {
                Op::If => {
                    let test = match self.known(a, EvalTy::Bool, &mut stack)? {
                        Ok(test) => test,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    match test {
                        Some(Value::Bool(true)) => stack.push((b, false)),
                        Some(Value::Bool(false)) => stack.push((c, false)),
                        Some(
                            Value::Int(_)
                            | Value::F32(_)
                            | Value::F64(_)
                            | Value::Enum(..)
                            | Value::Str(_)
                            | Value::Node(_)
                            | Value::Array(_)
                            | Value::Error,
                        )
                        | None => {}
                    }
                }
                Op::Index | Op::IndexFromEnd => match self.pick_operands(node)? {
                    Some(operands) => {
                        let mut values = Vec::new();
                        for operand in operands {
                            match self.known(operand, EvalTy::Exact, &mut stack)? {
                                Ok(Some(value)) => values.push(value),
                                Ok(None) => break,
                                Err(declined) => return Ok(Err(declined)),
                            }
                        }
                        let wanted = self
                            .pick_operands(node)?
                            .map_or(0, |operands| operands.len());
                        let Some(position) = values.last().filter(|_| values.len() == wanted)
                        else {
                            continue;
                        };
                        if op == Op::Index && self.index_outside_i64(b, position)? {
                            continue;
                        }
                        if op == Op::IndexFromEnd
                            && let Value::Int(offset) = position
                            && i64::try_from(*offset).is_err()
                        {
                            self.draw_at(
                                Code::IndexOutOfRange,
                                b,
                                outside_i64(I64Position::Offset, *offset),
                            )?;
                            continue;
                        }
                        let quiet = self.quiet_pick(node)?;
                        let mark = self.diagnostics.len();
                        let picked = match self.pick(node, &mut values)? {
                            Ok(picked) => picked,
                            Err(declined) => return Ok(Err(declined)),
                        };
                        if quiet && picked.is_none() {
                            self.diagnostics.truncate(mark);
                        }
                        if let Some(element) = picked {
                            stack.push((element, false));
                        }
                    }
                    None => {
                        if op == Op::IndexFromEnd {
                            match self.known(b, EvalTy::Exact, &mut stack)? {
                                Ok(Some(Value::Int(offset))) if i64::try_from(offset).is_err() => {
                                    self.draw_at(
                                        Code::IndexOutOfRange,
                                        b,
                                        outside_i64(I64Position::Offset, offset),
                                    )?;
                                }
                                Ok(_) => {}
                                Err(declined) => return Ok(Err(declined)),
                            }
                        } else {
                            stack.push((b, true));
                        }
                        stack.push((a, false));
                    }
                },
                Op::Slice => {
                    for bound in self.bounds(node)? {
                        if bound == NONE {
                            continue;
                        }
                        match self.known(bound, EvalTy::Exact, &mut stack)? {
                            Ok(Some(Value::Int(value))) if i64::try_from(value).is_err() => {
                                self.draw_at(
                                    Code::IndexOutOfRange,
                                    bound,
                                    outside_i64(I64Position::SliceBound, value),
                                )?;
                            }
                            Ok(_) => {}
                            Err(declined) => return Ok(Err(declined)),
                        }
                    }
                    stack.push((a, false));
                }
                Op::Name => {
                    if let DeclRef::Let(at) = self.bound(node)?
                        && !self.cyclic.get(at as usize).copied().unwrap_or(false)
                        && let Some(owned) = self.owned.get_mut(at as usize)
                        && !*owned
                    {
                        *owned = true;
                        stack.push((self.let_root(at)?, false));
                    }
                }
                Op::Int
                | Op::Float
                | Op::Bool
                | Op::Str
                | Op::Variant
                | Op::To
                | Op::Convert
                | Op::Field
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
                | Op::Array
                | Op::Match
                | Op::For
                | Op::TyScalar
                | Op::TyName
                | Op::TyArray
                | Op::TyVar
                | Op::Bad => {
                    for operand in self.operands(node)?.into_iter().rev() {
                        stack.push((operand, false));
                    }
                }
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
