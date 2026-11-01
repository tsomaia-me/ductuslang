use std::rc::Rc;

use symphonyscript_core::BinaryOp;

use super::convert::convert;
use super::convert_to::convert_to;
use super::ctx::Ctx;
use super::exact_binary::exact_binary;
use super::fetch_entry::FetchEntry;
use super::fetched::Fetched;
use super::instr::Instr;
use super::lay::Lay;
use super::loop_found::LoopFound;
use super::loop_items::LoopItems;
use super::program::Program;
use super::ran::Ran;
use super::run::{
    JOIN_SINGLE, SINGLE_OPERANDS, compare, fits_f32, float_binary, int_position, narrowed,
    scaled_into, wide_fits_f32,
};
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::fault_state::FaultState;
use crate::semantics::analysis::i64_position::I64Position;
use crate::semantics::analysis::map_leaves::map_leaves;
use crate::semantics::analysis::outside_i64::outside_i64;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::reachable::Reachable;
use crate::semantics::counted_walk::CountedWalk;

impl Engine<'_> {
    pub(super) fn run_program(
        &mut self,
        ctx: Ctx,
        program: &Rc<Program>,
        start: u32,
        need: &mut Vec<Query>,
    ) -> Result<Ran, Internal> {
        let mut stack = match ctx {
            Ctx::Frame(_) => std::mem::take(&mut self.machine_mut()?.stack),
            Ctx::Global => std::mem::take(&mut self.eval_stack),
        };
        let base = stack.len();
        let ran = self.execute(ctx, program, start as usize, &mut stack, need);
        match ctx {
            Ctx::Frame(_) => {
                if let Some(machine) = self.active.as_mut() {
                    machine.stack = stack;
                }
            }
            Ctx::Global => {
                stack.truncate(base);
                self.eval_stack = stack;
            }
        }
        ran
    }

    fn execute(
        &mut self,
        ctx: Ctx,
        program: &Program,
        start: usize,
        stack: &mut Vec<CVal>,
        need: &mut Vec<Query>,
    ) -> Result<Ran, Internal> {
        let cycles = self.cycles_known();
        let mut pc = start;
        while let Some(instr) = program.code.get(pc) {
            CountedWalk::ProgramStep.step();
            let at = u32::try_from(pc).map_err(|_| Internal::missed("program"))?;
            pc += 1;
            match instr {
                Instr::Push(value) => stack.push(value.clone()),
                Instr::Arg(slot) => {
                    let Ctx::Frame(frame) = ctx else {
                        return Err(Internal::missed("evaluation frame"));
                    };
                    stack.push(self.frame_arg(frame, *slot)?);
                }
                Instr::Slot(slot) => {
                    let Ctx::Frame(frame) = ctx else {
                        return Err(Internal::missed("evaluation frame"));
                    };
                    stack.push(self.slot_value(frame, *slot)?);
                }
                Instr::Child(node, ty) => match self.child(*node, *ty, ctx, need)? {
                    Some(value) => stack.push(value),
                    None => return Ok(Ran::Suspended(at)),
                },
                Instr::Fetch(entries) => {
                    if !self.fetch(entries, ctx, cycles, stack, need)? {
                        return Ok(Ran::Suspended(at));
                    }
                }
                Instr::Narrow(own) => {
                    let value = pop(stack)?;
                    stack.push(narrowed(value, *own)?);
                }
                Instr::Fault(code, node, message) => {
                    self.error_at_node(*code, *node, message.to_string())?;
                    stack.push(CVal::Error);
                }
                Instr::Draw(code, node, message) => {
                    self.error_at_node(*code, *node, message.to_string())?;
                }
                Instr::Convert(ty) => {
                    let value = pop(stack)?;
                    let value = match value {
                        CVal::Array(_) => {
                            let conversions = self.memo.conversions.entry(*ty).or_default();
                            map_leaves(value, conversions, |leaf| {
                                Ok::<CVal, Internal>(convert_to(leaf, *ty))
                            })?
                        }
                        leaf => convert_to(leaf, *ty),
                    };
                    stack.push(value);
                }
                Instr::Fit(node, ty) => {
                    let value = pop(stack)?;
                    stack.push(self.fit(*node, value, *ty)?);
                }
                Instr::Shape(node, shape) => {
                    let value = pop(stack)?;
                    stack.push(self.shape_held(*node, value, shape)?);
                }
                Instr::Scaled(node, scale, target) => {
                    let value = pop(stack)?;
                    stack.push(map_leaves(value, &mut FastMap::default(), |leaf| {
                        self.scaled_leaf(*node, *scale, leaf, *target)
                    })?);
                }
                Instr::ScaledInto(scale, own, ty) => {
                    let value = pop(stack)?;
                    stack.push(map_leaves(value, &mut FastMap::default(), |leaf| {
                        Ok::<CVal, Internal>(convert_to(scaled_into(leaf, *scale, *own)?, *ty))
                    })?);
                }
                Instr::BeforeUnit(node, ty) => {
                    let top = stack.last().ok_or(Internal::missed("program stack"))?;
                    self.memo
                        .before_unit_reading
                        .insert((*node, *ty), top.clone());
                }
                Instr::NarrowFloat(skip) => {
                    let wide = pop(stack)?;
                    if !wide_fits_f32(&wide) {
                        stack.push(wide);
                        pc = *skip as usize;
                    }
                }
                Instr::Unless(other, end) => match pop(stack)? {
                    CVal::Bool(true) => {}
                    CVal::Bool(false) => pc = *other as usize,
                    CVal::Int(_)
                    | CVal::F32(_)
                    | CVal::F64(_)
                    | CVal::Enum(..)
                    | CVal::Str(_)
                    | CVal::Node(_)
                    | CVal::Array(_)
                    | CVal::Error => {
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                },
                Instr::Jump(end) => pc = *end as usize,
                Instr::BailIfError(end) => {
                    if stack.last().is_some_and(CVal::is_error) {
                        pc = *end as usize;
                    }
                }
                Instr::Pick(index, targets, end) => {
                    let Some(position) = int_position(pop(stack)?) else {
                        stack.push(CVal::Error);
                        pc = *end as usize;
                        continue;
                    };
                    match usize::try_from(position)
                        .ok()
                        .and_then(|position| targets.get(position))
                    {
                        Some(target) => pc = *target as usize,
                        None => {
                            let length = u64::try_from(targets.len()).unwrap_or(u64::MAX);
                            self.index_outside(*index, position, length)?;
                            stack.push(CVal::Error);
                            pc = *end as usize;
                        }
                    }
                }
                Instr::MatchLabel(target) => {
                    let label = pop(stack)?;
                    if stack.last() == Some(&label) {
                        pop(stack)?;
                        pc = *target as usize;
                    }
                }
                Instr::Drop => {
                    pop(stack)?;
                }
                Instr::IfGlobal(global) => {
                    if ctx == Ctx::Global {
                        pc = *global as usize;
                    }
                }
                Instr::Element(base, index, lay) => {
                    let (value, position) = pair(stack, lay)?;
                    let value = match int_position(position) {
                        Some(position) => self.element(*base, *index, value, position)?,
                        None => CVal::Error,
                    };
                    stack.push(value);
                }
                Instr::Array(node, count, lay) => {
                    let values = operands(stack, *count as usize, lay)?;
                    stack.push(self.array_value(*node, values)?);
                }
                Instr::Not => {
                    let value = match pop(stack)? {
                        CVal::Bool(value) => CVal::Bool(!value),
                        CVal::Int(_)
                        | CVal::F32(_)
                        | CVal::F64(_)
                        | CVal::Enum(..)
                        | CVal::Str(_)
                        | CVal::Node(_)
                        | CVal::Array(_)
                        | CVal::Error => CVal::Error,
                    };
                    stack.push(value);
                }
                Instr::Neg(node, own) => {
                    let value = match pop(stack)? {
                        CVal::Int(value) => self.integer(*node, *own, value.checked_neg())?,
                        CVal::F32(value) => CVal::F32(-value),
                        CVal::F64(value) => CVal::F64(-value),
                        CVal::Bool(_)
                        | CVal::Enum(..)
                        | CVal::Str(_)
                        | CVal::Node(_)
                        | CVal::Array(_)
                        | CVal::Error => CVal::Error,
                    };
                    stack.push(value);
                }
                Instr::Logic(op, lay) => {
                    let value = match pair(stack, lay)? {
                        (CVal::Bool(a), CVal::Bool(b)) => {
                            CVal::Bool(if *op == BinaryOp::And { a && b } else { a || b })
                        }
                        _ => CVal::Error,
                    };
                    stack.push(value);
                }
                Instr::Arith(op, node, own, lay) => {
                    let value = match pair(stack, lay)? {
                        (CVal::Int(a), CVal::Int(b)) => {
                            self.integer(*node, *own, exact_binary(*op, a, b))?
                        }
                        (CVal::F32(a), CVal::F32(b)) => CVal::F32(
                            float_binary(*op, a, b).ok_or(Internal::missed("float operator"))?,
                        ),
                        (CVal::F64(a), CVal::F64(b)) => CVal::F64(
                            float_binary(*op, a, b).ok_or(Internal::missed("float operator"))?,
                        ),
                        _ => CVal::Error,
                    };
                    stack.push(value);
                }
                Instr::Compare(op, ty, a_fit, b_fit, lay) => {
                    let (a, b) = pair(stack, lay)?;
                    let a = match a_fit {
                        Some(node) => self.fit(*node, a, *ty)?,
                        None => a,
                    };
                    let b = match b_fit {
                        Some(node) => self.fit(*node, b, *ty)?,
                        None => b,
                    };
                    stack.push(if a.is_error() || b.is_error() {
                        CVal::Error
                    } else {
                        compare(*op, &a, &b)
                    });
                }
                Instr::FlexPair(narrow) => {
                    let b = pop(stack)?;
                    let a = pop(stack)?;
                    if fits_f32(&a) && fits_f32(&b) {
                        pc = *narrow as usize;
                    }
                }
                Instr::StrAdd(lay) => {
                    let value = match pair(stack, lay)? {
                        (CVal::Str(a), CVal::Str(b)) => {
                            CVal::Str(Rc::from(format!("{a}{b}").as_str()))
                        }
                        _ => CVal::Error,
                    };
                    stack.push(value);
                }
                Instr::IntDivFloat(node, own, flexible, lay) => {
                    let value = match pair(stack, lay)? {
                        (CVal::Int(a), CVal::Int(b)) => {
                            let value = self.integer(
                                *node,
                                EvalTy::Exact,
                                exact_binary(BinaryOp::IntDiv, a, b),
                            )?;
                            self.convert_integer(*node, *flexible, value, *own)?
                        }
                        _ => CVal::Error,
                    };
                    stack.push(value);
                }
                Instr::Div(posts, lay) => {
                    let (a, b) = pair(stack, lay)?;
                    let a = self.operand_post(posts.0, a)?;
                    let b = self.operand_post(posts.1, b)?;
                    stack.push(match (a, b) {
                        (CVal::F32(a), CVal::F32(b)) => CVal::F32(a / b),
                        (CVal::F64(a), CVal::F64(b)) => CVal::F64(a / b),
                        _ => CVal::Error,
                    });
                }
                Instr::FromEnd(rhs, end) => {
                    if let CVal::Int(value) = pop(stack)?
                        && i64::try_from(value).is_err()
                    {
                        self.error_at_node(
                            Code::IndexOutOfRange,
                            *rhs,
                            outside_i64(I64Position::Offset, value),
                        )?;
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                }
                Instr::DeferredCheck(node, end, lay, keep) => match pair_kept(stack, lay, *keep)? {
                    (CVal::Error, _) | (_, CVal::Error) => {
                        drop_kept(stack, *keep);
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                    (CVal::Array(_), _) | (_, CVal::Array(_)) => {
                        self.error_at_node(Code::ArrayOperand, *node, SINGLE_OPERANDS)?;
                        drop_kept(stack, *keep);
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                    (
                        CVal::Int(_)
                        | CVal::F32(_)
                        | CVal::F64(_)
                        | CVal::Bool(_)
                        | CVal::Enum(..)
                        | CVal::Str(_)
                        | CVal::Node(_),
                        _,
                    ) => {}
                },
                Instr::DeferredSum(node, end, lay, keep) => match pair_kept(stack, lay, *keep)? {
                    (CVal::Error, _) | (_, CVal::Error) => {
                        drop_kept(stack, *keep);
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                    (CVal::Array(a), CVal::Array(b)) => {
                        drop_kept(stack, *keep);
                        stack.push(self.joined(*node, &a, &b)?);
                        pc = *end as usize;
                    }
                    (CVal::Array(_), _) | (_, CVal::Array(_)) => {
                        self.error_at_node(Code::ArrayOperand, *node, JOIN_SINGLE)?;
                        drop_kept(stack, *keep);
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                    (CVal::Bool(_) | CVal::Enum(..) | CVal::Node(_), _)
                    | (_, CVal::Bool(_) | CVal::Enum(..) | CVal::Node(_)) => {
                        self.error_at_node(
                            Code::OperandType,
                            *node,
                            "`+` takes two numbers or two strings",
                        )?;
                        drop_kept(stack, *keep);
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                    (CVal::Int(_) | CVal::F32(_) | CVal::F64(_) | CVal::Str(_), _) => {}
                },
                Instr::ConvertTo(own) => {
                    let value = pop(stack)?;
                    stack.push(convert(value, *own));
                }
                Instr::Join(node, lay) => {
                    let value = match pair(stack, lay)? {
                        (CVal::Array(left), CVal::Array(right)) => {
                            self.joined(*node, &left, &right)?
                        }
                        (CVal::Error, _) | (_, CVal::Error) => CVal::Error,
                        _ => {
                            self.error_at_node(Code::ArrayOperand, *node, JOIN_SINGLE)?;
                            CVal::Error
                        }
                    };
                    stack.push(value);
                }
                Instr::Slice(base, lower, upper, lay) => {
                    let count = 1 + usize::from(lower.is_some()) + usize::from(upper.is_some());
                    let mut values = operands(stack, count, lay)?.into_iter();
                    let value = values.next().ok_or(Internal::missed("slice base"))?;
                    let start = match lower {
                        Some((bound, from_end)) => {
                            let at = values.next().ok_or(Internal::missed("slice bound"))?;
                            self.bound_position(*bound, at, *from_end)?
                        }
                        None => None,
                    };
                    let end = match upper {
                        Some((bound, from_end)) => {
                            let at = values.next().ok_or(Internal::missed("slice bound"))?;
                            self.bound_position(*bound, at, *from_end)?
                        }
                        None => None,
                    };
                    stack.push(self.slice_value(*base, value, start, end)?);
                }
                Instr::Length(operand) => {
                    let value = pop(stack)?;
                    stack.push(self.length_of(*operand, value)?);
                }
                Instr::Bytes(operand) => {
                    let value = pop(stack)?;
                    stack.push(self.bytes_of(*operand, value)?);
                }
                Instr::Capacity(operand, end) => {
                    if let Some(capacity) = self.extent(*operand)? {
                        stack.push(CVal::Int(i128::from(capacity)));
                        pc = *end as usize;
                    }
                }
                Instr::Rand(args, lay) => {
                    let values = operands(stack, args.len(), lay)?;
                    stack.push(self.rand_value(args, values)?);
                }
                Instr::Builtin(node, builtin, own, posts, lay) => {
                    let values = operands(stack, posts.len(), lay)?;
                    let mut posted = Vec::with_capacity(values.len());
                    for (value, post) in values.into_iter().zip(posts.iter()) {
                        posted.push(self.operand_post(*post, value)?);
                    }
                    let value = self
                        .builtin_value(*node, *builtin, *own, posted)?
                        .unwrap_or(CVal::Error);
                    stack.push(value);
                }
                Instr::Concat(parts, lay) => {
                    let count = parts
                        .iter()
                        .filter(|part| matches!(part, super::concat_part::ConcatPart::Value(..)))
                        .count();
                    let values = operands(stack, count, lay)?;
                    stack.push(self.concat_value(parts, values)?);
                }
                Instr::FitAnd(leaf, ty) => {
                    let value = pop(stack)?;
                    let fits = matches!(pop(stack)?, CVal::Bool(true));
                    let fitted = self.fit(*leaf, value, *ty)?;
                    stack.push(CVal::Bool(fits && !fitted.is_error()));
                }
                Instr::Gate(end) => {
                    if !matches!(pop(stack)?, CVal::Bool(true)) {
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                }
                Instr::Native(native, count, lay) => {
                    let values = operands(stack, *count as usize, lay)?;
                    stack.push(self.native_value(*native, values)?);
                }
                Instr::CallWith(call) => {
                    let count = call.count as usize;
                    let mut given = std::mem::take(&mut self.call_args);
                    peeked_into(stack, count, &call.lay, &mut given)?;
                    let called = self.call_with(
                        call.node,
                        call.function,
                        call.own,
                        ctx,
                        (&call.facts, &given, Some(&call.seen)),
                        need,
                    );
                    self.call_args = given;
                    let called = called?;
                    if let Ctx::Frame(_) = ctx {
                        let machine = self.machine_mut()?;
                        if machine.too_deep || machine.passed.is_some() {
                            return Ok(Ran::Abandoned);
                        }
                    }
                    match called {
                        Some(value) => {
                            stack.truncate(stack.len().saturating_sub(count));
                            stack.push(value);
                        }
                        None => return Ok(Ran::Suspended(at)),
                    }
                }
                Instr::LoopStart(node, own, end) => {
                    match self.loop_start(*node, *own, ctx, need)? {
                        LoopFound::Value(value) => {
                            stack.push(value);
                            pc = *end as usize;
                        }
                        LoopFound::Missing => {}
                        LoopFound::Pending => return Ok(Ran::Suspended(at)),
                    }
                }
                Instr::ForCheck(range, lay, end) => {
                    let count = if *range { 2 } else { 1 };
                    let mut values = Vec::new();
                    peeked_into(stack, count, lay, &mut values)?;
                    let valid = match values.as_slice() {
                        [CVal::Int(_), CVal::Int(_)] => *range,
                        [CVal::Array(_)] => !*range,
                        _ => false,
                    };
                    if !valid {
                        stack.truncate(stack.len().saturating_sub(count));
                        stack.push(CVal::Error);
                        pc = *end as usize;
                    }
                }
                Instr::ForRun(node, for_node, own, lay) => {
                    let range = matches!(
                        for_node.iterable,
                        crate::semantics::analysis::for_iterable::ForIterable::Range(..)
                    );
                    let count = if range { 3 } else { 2 };
                    if let LoopFound::Value(value) = self.loop_value(*node, *own, ctx)? {
                        stack.truncate(stack.len().saturating_sub(count));
                        stack.push(value);
                        continue;
                    }
                    let start = stack
                        .last()
                        .cloned()
                        .ok_or(Internal::missed("program stack"))?;
                    let below = stack.len().saturating_sub(1);
                    let mut values = Vec::new();
                    peeked_into(
                        stack
                            .get(..below)
                            .ok_or(Internal::missed("program stack"))?,
                        count - 1,
                        lay,
                        &mut values,
                    )?;
                    let items = match values.as_slice() {
                        [CVal::Int(start), CVal::Int(end)] => LoopItems::Range {
                            start: *start,
                            count: usize::try_from(end.saturating_sub(*start).max(0))
                                .unwrap_or(usize::MAX),
                        },
                        [CVal::Array(array)] => LoopItems::Array(array.clone()),
                        _ => return Err(Internal::missed("loop items")),
                    };
                    match self.for_run(*node, for_node, *own, (items, start), ctx, need)? {
                        Some(value) => {
                            stack.truncate(stack.len().saturating_sub(count));
                            stack.push(value);
                        }
                        None => return Ok(Ran::Suspended(at)),
                    }
                }
                Instr::FrameRead(read, own, flows) => {
                    let Ctx::Frame(frame) = ctx else {
                        return Err(Internal::missed("evaluation frame"));
                    };
                    let start = stack
                        .last()
                        .cloned()
                        .ok_or(Internal::missed("program stack"))?;
                    match self.frame_read(read, frame, start, (*own, *flows), need)? {
                        Some(value) => {
                            pop(stack)?;
                            stack.push(value);
                        }
                        None => return Ok(Ran::Suspended(at)),
                    }
                }
                Instr::ToRead(wire, item, attr, own) => {
                    match self.wire_read((*wire, *item, *attr), *own, ctx, need)? {
                        Some(value) => stack.push(value),
                        None => return Ok(Ran::Suspended(at)),
                    }
                }
            }
        }
        Ok(Ran::Value(pop(stack)?))
    }

    fn fetch(
        &mut self,
        entries: &[FetchEntry],
        ctx: Ctx,
        cycles: bool,
        stack: &mut Vec<CVal>,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let base = stack.len();
        let count = entries.len();
        stack.resize(base + count, CVal::Error);
        let mut missing = false;
        for (position, entry) in entries.iter().enumerate() {
            let value = match (cycles, entry.settled.get()) {
                (false, Some(value)) => Some(value.clone()),
                (true, _) | (false, None) => match entry.fetched {
                    Fetched::Raw(node, ty, narrow) => match self.child(node, ty, ctx, need)? {
                        Some(value) => {
                            let value = match narrow {
                                Some(own) => narrowed(value, own)?,
                                None => value,
                            };
                            if !cycles && self.settles(node, ty) {
                                let _ = entry.settled.set(value.clone());
                            }
                            Some(value)
                        }
                        None => None,
                    },
                    Fetched::Typed(node, own, flows) => {
                        self.typed_child(node, (own, flows), ctx, need)?
                    }
                },
            };
            match value {
                Some(value) => {
                    if let Some(slot) = stack.get_mut(base + count - 1 - position) {
                        *slot = value;
                    }
                }
                None => missing = true,
            }
        }
        if missing {
            stack.truncate(base);
        }
        Ok(!missing)
    }

    fn settles(&self, node: crate::semantics::analysis::ids::NodeId, ty: EvalTy) -> bool {
        self.memo.sigs.get(&node).is_some_and(|sig| !sig.frame)
            && self.memo.evals.contains_key(&(node, ty))
            && !matches!(
                self.memo.recorded.get(&Reachable::Eval(node, ty)),
                Some(FaultState::Recorded { .. })
            )
    }
}

fn pop(stack: &mut Vec<CVal>) -> Result<CVal, Internal> {
    stack.pop().ok_or(Internal::missed("program stack"))
}

fn pair(stack: &mut Vec<CVal>, lay: &Lay) -> Result<(CVal, CVal), Internal> {
    let top = pop(stack)?;
    let below = pop(stack)?;
    Ok(match lay {
        Lay::At(positions) if positions.first() == Some(&0) => (below, top),
        Lay::At(_) | Lay::Plain | Lay::Set(_) => (top, below),
    })
}

fn operands(stack: &mut Vec<CVal>, count: usize, lay: &Lay) -> Result<Vec<CVal>, Internal> {
    let start = stack
        .len()
        .checked_sub(count)
        .ok_or(Internal::missed("program stack"))?;
    let top = stack.split_off(start);
    Ok(ordered(top, lay))
}

fn peeked_into(
    stack: &[CVal],
    count: usize,
    lay: &Lay,
    into: &mut Vec<CVal>,
) -> Result<(), Internal> {
    let start = stack
        .len()
        .checked_sub(count)
        .ok_or(Internal::missed("program stack"))?;
    let top = stack
        .get(start..)
        .ok_or(Internal::missed("program stack"))?;
    into.clear();
    match lay {
        Lay::At(positions) => {
            for position in positions.iter() {
                into.push(
                    top.get(*position as usize)
                        .cloned()
                        .ok_or(Internal::missed("program stack"))?,
                );
            }
        }
        Lay::Plain | Lay::Set(_) => into.extend(top.iter().rev().cloned()),
    }
    Ok(())
}

fn ordered(mut top: Vec<CVal>, lay: &Lay) -> Vec<CVal> {
    match lay {
        Lay::At(positions) => positions
            .iter()
            .map(|position| {
                top.get_mut(*position as usize)
                    .map_or(CVal::Error, |slot| std::mem::replace(slot, CVal::Error))
            })
            .collect(),
        Lay::Plain | Lay::Set(_) => {
            top.reverse();
            top
        }
    }
}

fn pair_kept(stack: &mut Vec<CVal>, lay: &Lay, keep: bool) -> Result<(CVal, CVal), Internal> {
    if !keep {
        return pair(stack, lay);
    }
    let length = stack.len();
    let below = length
        .checked_sub(2)
        .ok_or(Internal::missed("program stack"))?;
    let (Some(lower), Some(top)) = (stack.get(below).cloned(), stack.get(below + 1).cloned())
    else {
        return Err(Internal::missed("program stack"));
    };
    Ok(match lay {
        Lay::At(positions) if positions.first() == Some(&0) => (lower, top),
        Lay::At(_) | Lay::Plain | Lay::Set(_) => (top, lower),
    })
}

fn drop_kept(stack: &mut Vec<CVal>, keep: bool) {
    if keep {
        stack.truncate(stack.len().saturating_sub(2));
    }
}
