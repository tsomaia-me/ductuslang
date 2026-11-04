use std::collections::HashMap;

use super::annotation::Refusal;
use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::fold_run::FoldRun;
use super::frame::Frame;
use super::leaf::Leaf;
use super::step::Step;
use super::width::{combined, is_fixed_float, target_ty, widens};
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::FloatLit;
use crate::semantics::ir::flat::{NONE, Op, binary_of};
use crate::semantics::operators::{
    EvalTy, I64Position, Operands, Operator, Value, convert, converted_integer, index_outside,
    int_range, outside_i64, plan_op, represents, ty_name,
};
use symphonyscript_core::BinaryOp;

pub(crate) fn class(ty: EvalTy) -> usize {
    match ty {
        EvalTy::F32 | EvalTy::DefaultF32 => 1,
        EvalTy::F64 | EvalTy::DefaultF64 => 2,
        EvalTy::Bool => 3,
        EvalTy::U8 => 4,
        EvalTy::I32 => 5,
        EvalTy::I64 => 6,
        EvalTy::Exact | EvalTy::Default | EvalTy::Enum(_) | EvalTy::Str | EvalTy::Node => 0,
    }
}

pub(crate) fn evaluated_in(ty: EvalTy) -> EvalTy {
    match class(ty) {
        1 => EvalTy::F32,
        2 => EvalTy::F64,
        3 => EvalTy::Bool,
        4 => EvalTy::U8,
        5 => EvalTy::I32,
        6 => EvalTy::I64,
        _ => EvalTy::Exact,
    }
}

pub(crate) fn flexible(ty: EvalTy) -> EvalTy {
    match class(ty) {
        4..=6 => EvalTy::Exact,
        _ => evaluated_in(ty),
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

    fn integral(&self, node: u32) -> Result<EvalTy, Internal> {
        Ok(match self.fact(node)?.own() {
            Some(fixed) if int_range(fixed).is_some() => fixed,
            Some(_) | None => EvalTy::Exact,
        })
    }

    fn division_ty(&self, a: u32, b: u32) -> Result<EvalTy, Internal> {
        for operand in [a, b] {
            let fact = self.fact(operand)?;
            let wide = match fact.own() {
                Some(own) => matches!(own, EvalTy::I64 | EvalTy::F64),
                None => {
                    fact.dims.is_empty()
                        && matches!(fact.leaf, Leaf::Int | Leaf::Float)
                        && self.is_reactive(operand)?
                }
            };
            if wide {
                return Ok(EvalTy::F64);
            }
        }
        Ok(EvalTy::F32)
    }

    fn pair_ty(&self, a: u32, b: u32) -> Result<EvalTy, Internal> {
        let (left, right) = (self.fact(a)?, self.fact(b)?);
        let float = left.leaf == Leaf::Float || right.leaf == Leaf::Float;
        if let Some(fixed) = combined(left.own(), right.own(), float) {
            return Ok(fixed);
        }
        let (x, y) = (left.leaf, right.leaf);
        Ok(match (x, y) {
            (Leaf::Bool, _) | (_, Leaf::Bool) => EvalTy::Bool,
            (Leaf::Float, _) | (_, Leaf::Float) => {
                let mut wide = false;
                for (operand, fact) in [(a, &left), (b, &right)] {
                    wide |= fact.leaf == Leaf::Float && self.is_reactive(operand)?;
                    wide |= self.own_float(operand)? == EvalTy::F64;
                }
                if wide { EvalTy::F64 } else { EvalTy::F32 }
            }
            _ => EvalTy::Exact,
        })
    }

    fn visit(&mut self, node: u32, ty: EvalTy, steps: &mut Vec<Step>) -> Result<(), Internal> {
        let (op, a, b, _) = self.ir.node(node)?;
        steps.push(Step::Apply(node, ty));
        let flows = !self.unflowed.remove(&node);
        if matches!(op, Op::Name | Op::Field) && !flows {
            self.unflowed.insert(node);
        }
        let float = matches!(ty, EvalTy::F32 | EvalTy::F64);
        let (mut integer_flow, mut picked) = (false, false);
        let operands: Vec<(u32, EvalTy)> = match op {
            Op::Neg | Op::Add | Op::Sub | Op::Mul | Op::Rem | Op::Pow => self
                .operands(node)?
                .into_iter()
                .map(|at| (at, ty))
                .collect(),
            Op::Div if !float => {
                let defaulted = self.fact(a)?.fixed.is_none() && self.fact(b)?.fixed.is_none();
                let target = self.division_ty(a, b)?;
                for operand in [b, a] {
                    self.unflowed.insert(operand);
                    let fact = self.fact(operand)?;
                    if defaulted && fact.dims.is_empty() && fact.leaf == Leaf::Int {
                        steps.push(Step::Float(operand, target));
                        steps.push(Step::Visit(operand, EvalTy::Exact));
                    } else if let Some(own) = self.own_width(operand, target, false)? {
                        steps.push(Step::Widen(target));
                        steps.push(Step::Visit(operand, own));
                    } else {
                        steps.push(Step::Visit(operand, target));
                    }
                }
                return Ok(());
            }
            Op::Div => vec![(a, ty), (b, ty)],
            Op::IntDiv => {
                let fixed = self.fact(node)?.fixed;
                let mut own = self.integral(node)?;
                let reaches = match fixed {
                    Some(fixed) => int_range(fixed).is_some() && widens(fixed, ty),
                    None => self.is_reactive(node)?,
                };
                if int_range(ty).is_some() && reaches {
                    own = ty;
                    integer_flow = true;
                }
                vec![(a, own), (b, own)]
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot => {
                let pair = self.pair_ty(a, b)?;
                if pair == EvalTy::Exact {
                    for operand in [b, a] {
                        self.unflowed.insert(operand);
                        let fact = self.fact(operand)?;
                        if fact.fixed.is_none() && fact.dims.is_empty() && fact.leaf == Leaf::Int {
                            steps.push(Step::Fit(operand, EvalTy::I64));
                        }
                        steps.push(Step::Visit(operand, pair));
                    }
                    return Ok(());
                }
                vec![(a, pair), (b, pair)]
            }
            Op::And | Op::Or => vec![(a, EvalTy::Bool), (b, EvalTy::Bool)],
            Op::Not => vec![(a, EvalTy::Bool)],
            Op::Call | Op::NamedCall => {
                steps.pop();
                steps.push(Step::Enter(node, ty));
                let mut operands = Vec::new();
                for arg in self.arguments(node)? {
                    let arg_ty = match self.fact(arg)?.fixed {
                        Some(_) => self.own_ty(arg)?.unwrap_or(EvalTy::Exact),
                        None => ty,
                    };
                    operands.push((arg, arg_ty));
                }
                operands
            }
            Op::Convert => {
                let (target, operand) = (self.fact(node)?.fixed, self.fact(a)?);
                if let Some(target) = target.filter(|target| is_fixed_float(*target))
                    && operand.fixed.is_none()
                    && operand.read.is_some_and(|read| int_range(read).is_some())
                    && operand.dims.is_empty()
                {
                    self.unflowed.insert(a);
                    steps.push(Step::Visit(a, target));
                    return Ok(());
                }
                let own = self.own_ty(a)?.unwrap_or(EvalTy::Exact);
                if own == EvalTy::Exact && self.fact(a)?.fixed.is_none() {
                    steps.push(Step::Fit(a, EvalTy::I64));
                }
                let divided = self.ir.node(a)?.0 == Op::Div
                    && operand.own().is_none()
                    && matches!(own, EvalTy::F32 | EvalTy::F64)
                    && !self.is_reactive(a)?;
                vec![(a, if divided { EvalTy::Exact } else { own })]
            }
            Op::If => {
                steps.pop();
                steps.push(Step::Choose(node, ty));
                if !flows {
                    self.unflowed.insert(node);
                }
                vec![(a, EvalTy::Bool)]
            }
            Op::Array => self
                .operands(node)?
                .into_iter()
                .map(|at| (at, ty))
                .collect(),
            Op::Index | Op::IndexFromEnd => match self.pick_operands(node)? {
                Some(operands) => {
                    picked = true;
                    steps.pop();
                    steps.push(Step::Pick(node, ty));
                    operands
                        .into_iter()
                        .map(|at| Ok((at, self.integral(at)?)))
                        .collect::<Result<_, Internal>>()?
                }
                None => vec![(a, ty), (b, self.integral(b)?)],
            },
            Op::Slice => {
                let mut operands = vec![(a, ty)];
                for bound in self.bounds(node)? {
                    if bound != NONE {
                        operands.push((bound, self.integral(bound)?));
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
            | Op::Field
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Past
            | Op::Stepped
            | Op::Match
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => Vec::new(),
            Op::For => {
                steps.pop();
                let own = self.fact(node)?.fixed.unwrap_or(ty);
                steps.push(Step::Fold(node, own));
                let rec = self.fold_rec(node)?;
                if rec.range {
                    vec![
                        (rec.iterable.0, EvalTy::Exact),
                        (rec.iterable.1, EvalTy::Exact),
                    ]
                } else {
                    let fact = self.fact(rec.iterable.0)?;
                    let element = usize::from(self.fold_names(node)? >= 2);
                    let items_ty = match (self.item_ty(&rec, element)?, fact.fixed) {
                        (Some(annotated), _) if fact.dims.len() == 1 => annotated,
                        (_, Some(fixed)) => fixed,
                        (_, None) if fact.leaf == Leaf::Bool => EvalTy::Bool,
                        (_, None) => EvalTy::Exact,
                    };
                    vec![(rec.iterable.0, items_ty)]
                }
            }
        };
        let arithmetic = matches!(
            op,
            Op::Neg | Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem | Op::Pow
        );
        let compared = matches!(op, Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot);
        let reactive = (arithmetic || compared || op == Op::IntDiv) && self.is_reactive(node)?;
        let runtime = reactive && !compared;
        let stopped = compared || (!flows && self.fact(node)?.own() == Some(ty));
        let mut through = Vec::with_capacity(operands.len());
        for (operand, _) in &operands {
            through.push(match op {
                Op::Neg | Op::Add | Op::Sub | Op::Mul | Op::Rem | Op::Pow | Op::Div | Op::Array => {
                    true
                }
                Op::IntDiv => integer_flow,
                Op::Index | Op::IndexFromEnd | Op::Slice => !picked && *operand == a,
                Op::Call | Op::NamedCall => self.fact(*operand)?.fixed.is_none(),
                Op::Int
                | Op::Float
                | Op::Bool
                | Op::Name
                | Op::Str
                | Op::Variant
                | Op::To
                | Op::Field
                | Op::MethodCall
                | Op::NamedMethodCall
                | Op::Past
                | Op::Stepped
                | Op::Match
                | Op::TyScalar
                | Op::TyName
                | Op::TyArray
                | Op::TyVar
                | Op::Bad
                | Op::And
                | Op::Or
                | Op::Not
                | Op::Lt
                | Op::Le
                | Op::Gt
                | Op::Ge
                | Op::Is
                | Op::IsNot
                | Op::Convert
                | Op::If
                | Op::For => false,
            });
        }
        for ((operand, operand_ty), through) in operands.into_iter().zip(through).rev() {
            if !(flows && through) {
                self.unflowed.insert(operand);
            }
            if runtime && self.compile_time_operand(operand, operand_ty)? {
                steps.push(Step::Fit(operand, operand_ty));
                steps.push(Step::Visit(operand, EvalTy::Exact));
                continue;
            }
            let apart = reactive && stopped && !self.is_reactive(operand)?;
            match self.own_width(operand, operand_ty, apart)? {
                Some(own) if arithmetic || apart => {
                    steps.push(Step::Widen(operand_ty));
                    steps.push(Step::Visit(operand, own));
                }
                Some(_) | None => steps.push(Step::Visit(operand, operand_ty)),
            }
        }
        Ok(())
    }

    fn compile_time_operand(&self, operand: u32, ty: EvalTy) -> Result<bool, Internal> {
        if int_range(ty).is_none() || self.is_reactive(operand)? || self.is_kept(operand)? {
            return Ok(false);
        }
        let fact = self.fact(operand)?;
        Ok(fact.fixed.is_none() && fact.dims.is_empty() && fact.leaf == Leaf::Int)
    }

    fn own_width(&self, operand: u32, ty: EvalTy, apart: bool) -> Result<Option<EvalTy>, Internal> {
        if !self.owning && !apart {
            return Ok(None);
        }
        let concrete = int_range(ty).is_some() || matches!(ty, EvalTy::F32 | EvalTy::F64);
        let fact = self.fact(operand)?;
        Ok(match fact.fixed {
            Some(own)
                if concrete
                    && own != ty
                    && widens(own, ty)
                    && fact.dims.is_empty()
                    && !matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) =>
            {
                Some(own)
            }
            Some(_) | None => None,
        })
    }

    pub fn evaluate(&mut self, root: u32, ty: EvalTy) -> Stage<CoreValue> {
        let inst = self.inst;
        let chain = self.eval_chain.clone();
        let outcome = self.evaluate_frames(root, ty);
        self.inst = inst;
        self.eval_chain = chain;
        outcome
    }

    fn enter_let_frame(&mut self, outer_chains: &mut Vec<Vec<u32>>) {
        if !self.eval_chain.is_empty() {
            self.unmemoized();
        }
        outer_chains.push(std::mem::take(&mut self.eval_chain));
    }

    fn evaluate_frames(&mut self, root: u32, ty: EvalTy) -> Stage<CoreValue> {
        let ty = evaluated_in(ty);
        let mut outer_chains: Vec<Vec<u32>> = Vec::new();
        let mut open: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut frames: Vec<Frame> = vec![Frame {
            owner: None,
            ty,
            steps: vec![Step::Visit(root, ty)],
            values: Vec::new(),
            call: None,
            inst: self.inst,
            fold: None,
        }];
        loop {
            let Some(frame) = frames.last_mut() else {
                return Err(Internal::missed("evaluation frame"));
            };
            self.inst = frame.inst;
            if frame.steps.is_empty()
                && let Some(run) = frame.fold.as_mut()
            {
                let value = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                let next = run.next + 1;
                let mut value = value;
                if !value.is_error() && next < run.count {
                    let proceed = match self.fold_step(run.node)? {
                        Ok(proceed) => proceed,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    if proceed {
                        let element = self.fold_item(run.node, &run.items, next)?;
                        run.next = next;
                        let mut args = Vec::new();
                        if run.indexed {
                            args.push(Value::Int(i128::try_from(next).unwrap_or(0)));
                        }
                        args.push(element);
                        args.push(value);
                        let body = self.fold_rec(run.node)?.body;
                        let ty = frame.ty;
                        frame.call = Some((run.node, args));
                        frame.values.clear();
                        frame.steps.push(Step::Visit(body, ty));
                        continue;
                    }
                    value = Value::Error;
                }
                if let Some(owner) = frame.owner {
                    open.remove(&owner);
                }
                frames.pop();
                CountedWalk::ValueRun.step();
                if let Some(parent) = frames.last_mut() {
                    parent.values.push(value);
                    continue;
                }
                return Ok(Ok(value));
            }
            let Some(step) = frame.steps.pop() else {
                let value = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                let (owner, frame_ty) = (frame.owner, frame.ty);
                let called = frame.call.is_some();
                if let Some(owner) = owner {
                    open.remove(&owner);
                }
                frames.pop();
                if owner.is_some()
                    && let Some(outer) = outer_chains.pop()
                {
                    self.eval_chain = outer;
                }
                CountedWalk::ValueRun.step();
                match owner {
                    Some(at) if at as usize >= self.ir.lets.len() => {
                        if let Some(slot) = self
                            .attr_values
                            .get_mut(at as usize - self.ir.lets.len())
                            .and_then(|slots| slots.get_mut(class(frame_ty)))
                        {
                            *slot = Some(value.clone());
                        }
                        if let Some(parent) = frames.last_mut() {
                            parent.values.push(value);
                        }
                    }
                    Some(at) => {
                        let value = match self.annotation_ty(at)? {
                            Some(annotated) => self.annotation_fit(at, annotated, value)?,
                            None => value,
                        };
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
                    None if called => {
                        self.eval_chain.pop();
                        self.end_recording(frames.len() + 1, &value);
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
                    if self.fitting
                        && matches!(op, Op::Name | Op::Field)
                        && self.is_reactive(node)?
                    {
                        if let Some(frame) = frames.last_mut() {
                            frame.values.push(Value::Error);
                        }
                        continue;
                    }
                    if op == Op::Field && self.bare_cell_read(node)?.is_some() {
                        let Some(frame) = frames.last_mut() else {
                            return Err(Internal::missed("evaluation frame"));
                        };
                        self.visit(node, node_ty, &mut frame.steps)?;
                        continue;
                    }
                    if op == Op::Field
                        && let Some(row) = self.path_rows.get(&node).copied()
                    {
                        let Some(frame) = frames.last_mut() else {
                            return Err(Internal::missed("evaluation frame"));
                        };
                        frame.values.push(Value::Node(row));
                        continue;
                    }
                    if op == Op::Field {
                        let read = self.path_read(node)?.cloned();
                        let plan = match read {
                            Some(read) if !read.cyclic && read.root == NONE => {
                                Some((self.default_read(&read)?, NONE, EvalTy::Exact))
                            }
                            Some(read) if !read.cyclic => {
                                let read_ty = match self.fact(read.root)?.fixed {
                                    Some(own) if widens(own, read.ty) => evaluated_in(read.ty),
                                    _ => flexible(read.ty),
                                };
                                Some((Value::Error, read.root, read_ty))
                            }
                            Some(_) | None => None,
                        };
                        let Some(frame) = frames.last_mut() else {
                            return Err(Internal::missed("evaluation frame"));
                        };
                        match plan {
                            Some((value, root, read_ty)) => {
                                if matches!(class(node_ty), 1 | 2 | 4..=6) {
                                    frame.steps.push(Step::Widen(evaluated_in(node_ty)));
                                }
                                let at = self.read_at(node)?;
                                let memo = self
                                    .attr_values
                                    .get(at as usize)
                                    .and_then(|slots| slots.get(class(read_ty)))
                                    .cloned()
                                    .flatten();
                                if root == NONE {
                                    frame.values.push(value);
                                } else if let Some(memo) = memo {
                                    frame.values.push(memo);
                                } else {
                                    self.enter_let_frame(&mut outer_chains);
                                    let owner = u32::try_from(self.ir.lets.len())
                                        .unwrap_or(u32::MAX)
                                        .saturating_add(at);
                                    open.insert(owner);
                                    frames.push(Frame {
                                        owner: Some(owner),
                                        ty: read_ty,
                                        steps: vec![Step::Visit(root, read_ty)],
                                        values: Vec::new(),
                                        call: None,
                                        inst: NONE,
                                        fold: None,
                                    });
                                }
                            }
                            None => frame.values.push(Value::Error),
                        }
                        continue;
                    }
                    if op == Op::Name {
                        match self.bound(node)? {
                            DeclRef::Param(position) => {
                                let owner = self.binding_owner(node)?;
                                let bodies = &self.bodies;
                                let owns = |open: &&Frame| {
                                    open.call.is_some()
                                        && bodies
                                            .get(open.inst as usize)
                                            .is_some_and(|body| body.function == owner)
                                };
                                let walked = || {
                                    self.own_calls
                                        .iter()
                                        .rev()
                                        .find(|(_, inst, _)| {
                                            bodies
                                                .get(*inst as usize)
                                                .is_some_and(|body| body.function == owner)
                                        })
                                        .map(|(_, _, args)| args)
                                };
                                let value = frames
                                    .iter()
                                    .rev()
                                    .find(owns)
                                    .and_then(|open| open.call.as_ref())
                                    .map(|(_, args)| args)
                                    .or_else(walked)
                                    .or_else(|| {
                                        frames
                                            .iter()
                                            .rev()
                                            .find_map(|open| open.call.as_ref())
                                            .map(|(_, args)| args)
                                    })
                                    .and_then(|args| args.get(position as usize).cloned())
                                    .unwrap_or(Value::Error);
                                let value = match self
                                    .loop_element(node, owner, position, node_ty, &value)?
                                {
                                    Ok(value) => value,
                                    Err(declined) => return Ok(Err(declined)),
                                };
                                if let Some(frame) = frames.last_mut() {
                                    frame.values.push(value);
                                }
                                continue;
                            }
                            DeclRef::Let(at)
                                if self
                                    .ir
                                    .lets
                                    .get(at as usize)
                                    .is_some_and(|rec| rec.owner != NONE) =>
                            {
                                let root = self.let_root(at)?;
                                let cyclic = self.cyclic.get(at as usize).copied().unwrap_or(false);
                                if self.unflowed.remove(&node) && !cyclic {
                                    self.unflowed.insert(root);
                                }
                                if let Some(frame) = frames.last_mut() {
                                    if cyclic {
                                        frame.values.push(Value::Error);
                                    } else {
                                        frame.steps.push(Step::Visit(root, node_ty));
                                    }
                                }
                                continue;
                            }
                            DeclRef::None
                            | DeclRef::Let(_)
                            | DeclRef::Cell(_)
                            | DeclRef::Instance(_)
                            | DeclRef::Duplicate
                            | DeclRef::Fn(_) => {}
                        }
                    }
                    let Some(frame) = frames.last_mut() else {
                        return Err(Internal::missed("evaluation frame"));
                    };
                    if op == Op::Name
                        && let DeclRef::Let(at) = self.bound(node)?
                        && !self.cyclic.get(at as usize).copied().unwrap_or(false)
                    {
                        let top = self
                            .ir
                            .lets
                            .get(at as usize)
                            .is_some_and(|rec| rec.owner == NONE);
                        if top && self.let_reads_cell(at)? {
                            if !self.fitting
                                && !self.owning
                                && let Some(owned) = self.owned.get_mut(at as usize)
                                && !*owned
                            {
                                *owned = true;
                                self.unmemoized();
                                if let Err(declined) = self.operand_fits(at)? {
                                    return Ok(Err(declined));
                                }
                            }
                            frame.values.push(Value::Error);
                            continue;
                        }
                        if let Some(annotated) = self.annotation_ty(at)? {
                            let let_ty = match self.fact(self.let_root(at)?)?.fixed {
                                Some(own) if widens(own, annotated) => evaluated_in(annotated),
                                _ if !self.annotation_dims(at)?.is_empty() => {
                                    evaluated_in(annotated)
                                }
                                _ => flexible(annotated),
                            };
                            if matches!(class(node_ty), 1 | 2 | 4..=6) {
                                frame.steps.push(Step::Widen(evaluated_in(node_ty)));
                            }
                            if let Some(value) = self.memo(at, let_ty) {
                                frame.values.push(value);
                                continue;
                            }
                            if self.annotation_refusal(at, annotated)? != Refusal::Kept {
                                frame.values.push(Value::Error);
                                continue;
                            }
                            if open.contains(&at) {
                                return Err(Internal::missed(
                                    "an open declaration outside every cycle",
                                ));
                            }
                            let root = self.let_root(at)?;
                            self.enter_let_frame(&mut outer_chains);
                            open.insert(at);
                            frames.push(Frame {
                                owner: Some(at),
                                ty: let_ty,
                                steps: vec![Step::Visit(root, let_ty)],
                                values: Vec::new(),
                                call: None,
                                inst: NONE,
                                fold: None,
                            });
                            continue;
                        }
                        frame.steps.push(Step::Record(node, false, at, node_ty));
                        let flexible_let = self.fact(self.let_root(at)?)?.fixed.is_none();
                        if flexible_let && (4..=6).contains(&class(node_ty)) {
                            frame.steps.push(Step::Fit(node, node_ty));
                        }
                        if let Some(value) = self.memo(at, node_ty) {
                            frame.values.push(value);
                            continue;
                        }
                        if open.contains(&at) {
                            return Err(Internal::missed(
                                "an open declaration outside every cycle",
                            ));
                        }
                        let let_ty = if self.fact(self.let_root(at)?)?.fixed.is_some() {
                            evaluated_in(node_ty)
                        } else {
                            flexible(node_ty)
                        };
                        let root = self.let_root(at)?;
                        self.enter_let_frame(&mut outer_chains);
                        open.insert(at);
                        frames.push(Frame {
                            owner: Some(at),
                            ty: let_ty,
                            steps: vec![Step::Visit(root, let_ty)],
                            values: Vec::new(),
                            call: None,
                            inst: NONE,
                            fold: None,
                        });
                        continue;
                    }
                    let mut steps = std::mem::take(&mut frame.steps);
                    self.visit(node, node_ty, &mut steps)?;
                    if let Some(frame) = frames.last_mut() {
                        frame.steps = steps;
                    }
                }
                Step::Choose(choice, arm_ty) => {
                    let (_, condition, then, otherwise) = self.ir.node(choice)?;
                    let unflowed = self.unflowed.remove(&choice);
                    let test = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                    let held = self.fact(condition)?;
                    let test = if held.leaf == Leaf::Bool && held.dims.is_empty() {
                        test
                    } else {
                        Value::Error
                    };
                    match test {
                        Value::Bool(true) => {
                            if unflowed {
                                self.unflowed.insert(then);
                            }
                            frame.steps.push(Step::Visit(then, arm_ty));
                        }
                        Value::Bool(false) => {
                            if unflowed {
                                self.unflowed.insert(otherwise);
                            }
                            frame.steps.push(Step::Visit(otherwise, arm_ty));
                        }
                        Value::Error if self.is_reactive(condition)? => {
                            if unflowed {
                                self.unflowed.extend([then, otherwise]);
                            }
                            frame.steps.push(Step::Runtime(2));
                            frame.steps.push(Step::Visit(otherwise, arm_ty));
                            frame.steps.push(Step::Visit(then, arm_ty));
                        }
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
                Step::Fold(fold, fold_ty) => {
                    let rec = self.fold_rec(fold)?;
                    let count = if rec.range { 2 } else { 1 };
                    let keep = frame
                        .values
                        .len()
                        .checked_sub(count)
                        .ok_or(Internal::missed("value stack"))?;
                    let valid = match frame.values.get(keep..) {
                        Some([Value::Int(_), Value::Int(_)]) => rec.range,
                        Some([Value::Array(_)]) => !rec.range,
                        Some(_) | None => false,
                    };
                    if valid && !self.annotated_fold_refused(fold)? {
                        frame.steps.push(Step::FoldRun(fold, fold_ty));
                        frame.steps.push(Step::Visit(rec.start, fold_ty));
                    } else {
                        frame.values.truncate(keep);
                        frame.values.push(Value::Error);
                    }
                }
                Step::FoldRun(fold, fold_ty) => {
                    let rec = self.fold_rec(fold)?;
                    let start = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                    let (items, count) = if rec.range {
                        let high = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                        let low = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                        match (low, high) {
                            (Value::Int(low), Value::Int(high)) => (
                                Value::Int(low),
                                usize::try_from((high - low).max(0)).unwrap_or(usize::MAX),
                            ),
                            _ => (Value::Error, 0),
                        }
                    } else {
                        let items = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                        let count = match &items {
                            Value::Array(array) => self.store.len(*array) as usize,
                            _ => 0,
                        };
                        (items, count)
                    };
                    if start.is_error() || count == 0 {
                        frame.values.push(start);
                        continue;
                    }
                    let Some(body) = self.body_of_call(fold) else {
                        frame.values.push(Value::Error);
                        continue;
                    };
                    let proceed = match self.fold_step(fold)? {
                        Ok(proceed) => proceed,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    if !proceed {
                        frame.values.push(Value::Error);
                        continue;
                    }
                    let indexed = self.fold_names(fold)? >= 2;
                    let first = self.fold_item(fold, &items, 0)?;
                    let mut args = Vec::new();
                    if indexed {
                        args.push(Value::Int(0));
                    }
                    args.push(first);
                    args.push(start);
                    frames.push(Frame {
                        owner: None,
                        ty: fold_ty,
                        steps: vec![Step::Visit(rec.body, fold_ty)],
                        values: Vec::new(),
                        call: Some((fold, args)),
                        inst: body,
                        fold: Some(FoldRun {
                            node: fold,
                            items,
                            next: 0,
                            count,
                            indexed,
                        }),
                    });
                }
                Step::Enter(call, call_ty) => {
                    let count = self.arguments(call)?.len();
                    let start = frame
                        .values
                        .len()
                        .checked_sub(count)
                        .ok_or(Internal::missed("value stack"))?;
                    let args = frame.values.split_off(start);
                    let Some(body) = self.body_of_call(call) else {
                        frame.values.push(Value::Error);
                        continue;
                    };
                    let key = self.memo_key(body, call_ty, &args);
                    if let Some(key) = key.as_ref()
                        && let Some(value) = self.memo_hit(key, call)
                    {
                        frame.values.push(value);
                        continue;
                    }
                    let value = self.body_value(body)?;
                    if let Some(key) = key {
                        self.begin_recording(key, frames.len() + 1);
                    }
                    self.eval_chain.push(call);
                    frames.push(Frame {
                        owner: None,
                        ty: call_ty,
                        steps: vec![Step::Visit(value, call_ty)],
                        values: Vec::new(),
                        call: Some((call, args)),
                        inst: body,
                        fold: None,
                    });
                }
                Step::Runtime(count) => {
                    let keep = frame
                        .values
                        .len()
                        .checked_sub(count as usize)
                        .ok_or(Internal::missed("value stack"))?;
                    frame.values.truncate(keep);
                    frame.values.push(Value::Error);
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
                            let (_, base, _, _) = self.ir.node(read)?;
                            let runtime = self.is_reactive(base)? && !self.is_reactive(element)?;
                            match self.own_ty(read)?.filter(|_| runtime) {
                                Some(own) => {
                                    frame.steps.push(Step::Fit(read, element_ty));
                                    frame.steps.push(Step::Visit(element, own));
                                }
                                None => frame.steps.push(Step::Visit(element, element_ty)),
                            }
                        }
                        None => {
                            let (_, base, index, _) = self.ir.node(read)?;
                            match self.view(base)? {
                                Some((array, _)) if self.is_reactive(index)? => {
                                    let (_, start, count, _) = self.ir.node(array)?;
                                    frame.steps.push(Step::Runtime(count));
                                    for at in (start..start + count).rev() {
                                        frame.steps.push(Step::Visit(self.kid(at)?, element_ty));
                                    }
                                }
                                Some(_) | None => frame.values.push(Value::Error),
                            }
                        }
                    }
                }
                Step::Widen(target) => {
                    if let Some(value) = frame.values.pop() {
                        frame.values.push(convert(value, target));
                    }
                }
                Step::Float(operand, target) => {
                    if let Some(value) = frame.values.pop() {
                        frame
                            .values
                            .push(converted_integer(self, operand, true, value, target)?);
                    }
                }
                Step::Fit(read, read_ty) => {
                    if let Some(Value::Int(value)) = frame.values.last()
                        && let Some(range) = int_range(read_ty)
                        && !range.contains(value)
                    {
                        frame.values.pop();
                        frame.values.push(Value::Error);
                        self.draw_at(
                            Code::DoesNotFit,
                            read,
                            format!("this value does not fit `{}`", ty_name(read_ty)),
                        )?;
                    }
                }
                Step::Record(read, element, target, read_ty) => {
                    if let Some(value) = frame.values.last()
                        && !value.is_error()
                        && !matches!(value, Value::Array(_))
                    {
                        let value = value.clone();
                        self.uses
                            .entry((read, class(read_ty), self.eval_chain.clone()))
                            .or_insert(((element, target), value));
                    }
                }
                Step::Apply(node, node_ty) => {
                    let mut values = std::mem::take(&mut frame.values);
                    let value = match self.apply(node, node_ty, &mut values)? {
                        Ok(value) => value,
                        Err(declined) => return Ok(Err(declined)),
                    };
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
        let unflowed = matches!(op, Op::Name | Op::Field) && self.unflowed.remove(&node);
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
                DeclRef::Cell(at) => match self.read_cell(at, node, ty, unflowed)? {
                    Ok(value) => value,
                    Err(declined) => return Ok(Err(declined)),
                },
                DeclRef::Duplicate | DeclRef::None | DeclRef::Param(_) | DeclRef::Fn(_) => {
                    Value::Error
                }
                DeclRef::Instance(item) => match self.row_of_item.get(item as usize).copied() {
                    Some(row) if row != NONE => Value::Node(row),
                    Some(_) | None => Value::Error,
                },
            },
            Op::Neg => {
                let operand = pop(stack)?;
                plan_op(self, Operator::Neg(node, ty), Operands::One(operand))?
            }
            Op::Convert => {
                let operand = pop(stack)?;
                let target = target_ty(b).ok_or(Internal::missed("a conversion target"))?;
                plan_op(self, Operator::ConvertTo(target), Operands::One(operand))?
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
                self.runtime_outside((node, a, b), (&array, &position), I64Position::Index)?;
                plan_op(
                    self,
                    Operator::Element(a, b),
                    Operands::Two(array, position),
                )?
            }
            Op::IndexFromEnd => {
                let offset = pop(stack)?;
                let array = pop(stack)?;
                self.runtime_outside((node, a, b), (&array, &offset), I64Position::Offset)?;
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
            Op::Field => match self.bare_cell_read(node)? {
                Some(at) => match self.read_cell(at, node, ty, unflowed)? {
                    Ok(value) => value,
                    Err(declined) => return Ok(Err(declined)),
                },
                None => return Err(Internal::missed("a path read of a cell")),
            },
            Op::If
            | Op::Str
            | Op::Variant
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
            | Op::TyVar => return Err(Internal::missed("an op the coverage table leaves out")),
            Op::Bad | Op::To => Value::Error,
        }))
    }

    fn read_cell(&mut self, at: u32, node: u32, ty: EvalTy, unflowed: bool) -> Stage<CoreValue> {
        if self.fitting {
            return Ok(Ok(Value::Error));
        }
        if let Some(rec) = self.ir.cells.get(at as usize).copied()
            && rec.rule != NONE
            && let Some(done) = self
                .rule_runs
                .get_mut(at as usize)
                .and_then(|runs| runs.get_mut(class(ty)))
            && !*done
        {
            *done = true;
            if unflowed {
                self.unflowed.insert(rec.rule);
            }
            if let Err(declined) = self.evaluate(rec.rule, ty)? {
                return Ok(Err(declined));
            }
        }
        if !self.owning
            && self.cell_annotation_ty(at)?.is_none()
            && let Some(range) = int_range(ty)
            && let Some(rec) = self.ir.cells.get(at as usize).copied()
            && rec.rule == NONE
        {
            let init = match self.evaluate(rec.init, EvalTy::Exact)? {
                Ok(init) => init,
                Err(declined) => return Ok(Err(declined)),
            };
            if let Value::Int(init) = init
                && !range.contains(&init)
            {
                let name = self.ir.binder(rec.binder)?.name;
                let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
                let text = self.ir.symbols.text(entry.sym).to_string();
                self.draw_at(
                    Code::DoesNotFit,
                    node,
                    format!(
                        "the initial value of `{text}` does not fit the type this use reads it in"
                    ),
                )?;
            }
        }
        let side = match class(ty) {
            _ if self.owning => Some(0),
            1 | 2 => Some(1),
            3 => None,
            _ => Some(0),
        };
        if let Some(side) = side
            && let Some(seen) = self
                .cell_uses
                .get_mut(at as usize)
                .and_then(|uses| uses.get_mut(side))
        {
            *seen = true;
        }
        Ok(Ok(Value::Error))
    }

    fn runtime_outside(
        &mut self,
        (read, base, index): (u32, u32, u32),
        (array, position): (&CoreValue, &CoreValue),
        at: I64Position,
    ) -> Result<(), Internal> {
        let Value::Int(value) = position else {
            return Ok(());
        };
        if !array.is_error() || !self.is_reactive(base)? {
            return Ok(());
        }
        let Some(length) = self.fact(base)?.dims.first().copied() else {
            return Ok(());
        };
        let (position, drawn) = if at == I64Position::Offset {
            if i64::try_from(*value).is_err() {
                self.draw_at(Code::IndexOutOfRange, index, outside_i64(at, *value))?;
                return Ok(());
            }
            (i128::from(length) - value, read)
        } else {
            (*value, index)
        };
        if !(0..i128::from(length)).contains(&position) {
            index_outside(self, drawn, position, u64::from(length))?;
        }
        Ok(())
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

    pub fn own_ty(&self, node: u32) -> Result<Option<EvalTy>, Internal> {
        let fact = self.fact(node)?;
        if let Some(fixed) = fact.own()
            && !matches!(fact.leaf, Leaf::Placeholder | Leaf::Error)
        {
            return Ok(Some(fixed));
        }
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
        self.evaluate_own_in(root, None)
    }

    pub fn evaluate_own_in(&mut self, root: u32, expected: Option<EvalTy>) -> Stage<()> {
        let owning = std::mem::replace(&mut self.owning, true);
        let outcome = self.evaluate_owned(root, expected);
        self.owning = owning;
        outcome
    }

    fn evaluate_owned(&mut self, root: u32, expected: Option<EvalTy>) -> Stage<()> {
        let inst = self.inst;
        let chain = self.eval_chain.len();
        let calls = self.own_calls.len();
        let outcome = self.own_walk(root, expected);
        self.inst = inst;
        self.eval_chain.truncate(chain);
        self.own_calls.truncate(calls);
        outcome
    }

    fn own_walk(&mut self, root: u32, expected: Option<EvalTy>) -> Stage<()> {
        let mut stack = vec![(root, false)];
        let mut leaves: Vec<u32> = Vec::new();
        let mut fits: Vec<u32> = Vec::new();
        let mut unfixed = HashMap::new();
        let mut flowing: HashMap<(u32, u32), EvalTy> = HashMap::new();
        if let Some(ty) = expected {
            flowing.insert((self.inst, root), ty);
        }
        while let Some((node, indexed)) = stack.pop() {
            if node == NONE {
                self.eval_chain.pop();
                self.own_calls.pop();
                self.inst = leaves.pop().ok_or(Internal::missed("a call to leave"))?;
                continue;
            }
            let fact = self.fact(node)?;
            let own = self.own_ty(node)?;
            let (op, a, b, c) = self.ir.node(node)?;
            if op == Op::Convert
                && let Some(target) = fact.fixed.filter(|target| is_fixed_float(*target))
                && self.is_reactive(a)?
            {
                let operand = self.fact(a)?;
                if operand.fixed.is_none()
                    && operand.read.is_some_and(|read| int_range(read).is_some())
                    && operand.dims.is_empty()
                {
                    self.unflowed.insert(a);
                    let owning = std::mem::replace(&mut self.owning, false);
                    let outcome = self.evaluate(a, target);
                    self.owning = owning;
                    if let Err(declined) = outcome? {
                        return Ok(Err(declined));
                    }
                    continue;
                }
            }
            if let Some(own) = own
                && !self.is_kept(node)?
                && (!self.is_reactive(node)? || self.runtime_in_own(node, own, &mut unfixed)?)
            {
                let flowed = match flowing.get(&(self.inst, node)).copied() {
                    Some(ty)
                        if self.is_reactive(node)?
                            && (widens(own, ty)
                                || (fact.fixed.is_none()
                                    && int_range(own).is_some()
                                    && int_range(ty).is_some())) =>
                    {
                        Some(ty)
                    }
                    Some(_) | None => None,
                };
                if flowed.is_none() {
                    self.unflowed.insert(node);
                }
                let divided = op == Op::Div
                    && fact.own().is_none()
                    && matches!(own, EvalTy::F32 | EvalTy::F64);
                let evaluated = if let Some(ty) = flowed {
                    ty
                } else if !divided {
                    own
                } else if self.is_reactive(node)? {
                    self.division_ty(a, b)?
                } else {
                    EvalTy::Exact
                };
                let value = match self.evaluate(node, evaluated)? {
                    Ok(value) => value,
                    Err(declined) => return Ok(Err(declined)),
                };
                if indexed && fact.dims.is_empty() {
                    self.index_outside_i64(node, &value)?;
                }
                continue;
            }
            match op {
                Op::If => {
                    if let Some(ty) = flowing.get(&(self.inst, node)).copied() {
                        flowing.insert((self.inst, b), ty);
                        flowing.insert((self.inst, c), ty);
                    }
                    let condition = self.fact(a)?;
                    if condition.dims.is_empty()
                        && !matches!(condition.leaf, Leaf::Bool | Leaf::Placeholder | Leaf::Error)
                    {
                        if self.is_reactive(a)? {
                            stack.push((c, false));
                            stack.push((b, false));
                        }
                        stack.push((a, false));
                        continue;
                    }
                    let test = match self.known(a, EvalTy::Bool, &mut stack)? {
                        Ok(test) => test,
                        Err(declined) => return Ok(Err(declined)),
                    };
                    match test {
                        Some(Value::Bool(true)) => stack.push((b, false)),
                        Some(Value::Bool(false)) => stack.push((c, false)),
                        _ if self.is_reactive(a)?
                            && !matches!(self.fact(a)?.leaf, Leaf::Placeholder | Leaf::Error) =>
                        {
                            stack.push((c, false));
                            stack.push((b, false));
                        }
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
                            let runtime = self.is_reactive(a)? && !self.is_reactive(element)?;
                            match self.own_ty(node)?.filter(|_| runtime) {
                                Some(own) => {
                                    self.unflowed.insert(element);
                                    if let Err(declined) = self.evaluate(element, own)? {
                                        return Ok(Err(declined));
                                    }
                                }
                                None => stack.push((element, false)),
                            }
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
                        let bound_ty = match self.fact(bound)?.own() {
                            Some(_) => self.own_ty(bound)?.unwrap_or(EvalTy::Exact),
                            None => EvalTy::Exact,
                        };
                        match self.known(bound, bound_ty, &mut stack)? {
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
                Op::Call | Op::NamedCall => {
                    let mut args = Vec::new();
                    for arg in self.arguments(node)? {
                        let held = match self.own_ty(arg)? {
                            Some(own) if !self.is_kept(arg)? && !self.is_reactive(arg)? => {
                                self.unflowed.insert(arg);
                                match self.evaluate(arg, own)? {
                                    Ok(value) => Some(value),
                                    Err(declined) => return Ok(Err(declined)),
                                }
                            }
                            Some(_) | None => None,
                        };
                        if held.is_none() {
                            stack.push((arg, false));
                        }
                        args.push(held.unwrap_or(Value::Error));
                    }
                    if let Some(body) = self.body_of_call(node) {
                        let value = self.body_value(body)?;
                        leaves.push(std::mem::replace(&mut self.inst, body));
                        self.eval_chain.push(node);
                        self.own_calls.push((node, body, args));
                        stack.push((NONE, false));
                        stack.push((value, false));
                    }
                }
                Op::Name => {
                    if let DeclRef::Let(at) = self.bound(node)?
                        && !self.cyclic.get(at as usize).copied().unwrap_or(false)
                        && let Some(owned) = self.owned.get_mut(at as usize)
                        && !*owned
                    {
                        *owned = true;
                        self.unmemoized();
                        fits.push(at);
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
        for at in fits {
            if let Err(declined) = self.operand_fits(at)? {
                return Ok(Err(declined));
            }
        }
        Ok(Ok(()))
    }

    fn runtime_in_own(
        &self,
        node: u32,
        own: EvalTy,
        unfixed: &mut HashMap<(u32, u32), bool>,
    ) -> Result<bool, Internal> {
        let (op, ..) = self.ir.node(node)?;
        if int_range(own).is_some() {
            return Ok(matches!(
                op,
                Op::Neg | Op::Add | Op::Sub | Op::Mul | Op::Rem | Op::Pow | Op::IntDiv
            ));
        }
        let computed = match own {
            EvalTy::F32 | EvalTy::F64 => matches!(
                op,
                Op::Neg | Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem | Op::Pow
            ),
            EvalTy::Bool => matches!(op, Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot),
            EvalTy::Exact
            | EvalTy::Default
            | EvalTy::U8
            | EvalTy::I32
            | EvalTy::I64
            | EvalTy::DefaultF32
            | EvalTy::DefaultF64
            | EvalTy::Enum(_)
            | EvalTy::Str
            | EvalTy::Node => false,
        };
        Ok(computed && !self.reads_unfixed_number(node, unfixed)?)
    }

    fn reads_unfixed_number(
        &self,
        node: u32,
        unfixed: &mut HashMap<(u32, u32), bool>,
    ) -> Result<bool, Internal> {
        if let Some(found) = unfixed.get(&(self.inst, node)) {
            return Ok(*found);
        }
        let mut kids = Vec::new();
        for at in self.postorder(node)? {
            if unfixed.contains_key(&(self.inst, at)) {
                continue;
            }
            let fact = self.fact(at)?;
            let read = self.ir.node(at)?.0 == Op::Name
                && matches!(fact.leaf, Leaf::Float | Leaf::Int)
                && self.is_reactive(at)?
                && match self.bound(at)? {
                    DeclRef::Cell(cell) => self.cell_annotation_ty(cell)?.is_none(),
                    DeclRef::Let(_)
                    | DeclRef::Param(_)
                    | DeclRef::None
                    | DeclRef::Instance(_)
                    | DeclRef::Duplicate
                    | DeclRef::Fn(_) => fact.own().is_none(),
                };
            kids.clear();
            self.push_operands(at, &mut kids)?;
            let found = read
                || kids
                    .iter()
                    .any(|kid| unfixed.get(&(self.inst, *kid)).copied().unwrap_or(false));
            unfixed.insert((self.inst, at), found);
        }
        Ok(unfixed.get(&(self.inst, node)).copied().unwrap_or(false))
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
                if let Some(range) = int_range(ty)
                    && !range.contains(&value)
                {
                    self.draw_at(
                        Code::DoesNotFit,
                        node,
                        format!("this value does not fit `{}`", ty_name(ty)),
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
