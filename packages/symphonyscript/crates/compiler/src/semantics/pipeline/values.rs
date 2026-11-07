use super::annotation::Refusal;
use super::call_chain::CallChain;
use super::core::{Core, Stage};
use super::core_value::CoreValue;
use super::decl_ref::DeclRef;
use super::fold_run::FoldRun;
use super::folds::ANY_LENGTH;
use super::frame::Frame;
use super::leaf::Leaf;
use super::step::Step;
use super::unit_reach::FrameUnit;
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

fn merge_reach(
    scaled: &mut super::column::ColumnSet,
    merged: &mut super::column::TripleSet,
    unit: Option<&FrameUnit>,
) {
    if let Some(unit) = unit
        && merged.insert(unit.key)
    {
        scaled.extend(unit.reach.scaled.iter());
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

    fn let_memo(
        &self,
        at: u32,
        ty: EvalTy,
        unit: Option<(symphonyscript_core::FamilyId, usize)>,
    ) -> Option<CoreValue> {
        match unit {
            Some((family, unit)) => self
                .unit_let_values
                .get(&(at, class(ty), family, unit))
                .cloned(),
            None => self.memo(at, ty),
        }
    }

    fn integral(&self, node: u32) -> Result<EvalTy, Internal> {
        let fact = self.fact(node)?;
        let divided = fact.own().is_none() && self.ir.node(node)?.0 == Op::Div;
        let float = fact.leaf == Leaf::Float && fact.dims.is_empty() && !divided;
        Ok(match fact.own() {
            Some(fixed) if int_range(fixed).is_some() => fixed,
            Some(_) | None if float => self.own_ty(node)?.unwrap_or(EvalTy::Exact),
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
                let (mut host, mut derived, mut argument, mut literal) =
                    (false, false, false, false);
                for (operand, fact) in [(a, &left), (b, &right)] {
                    let (op, x, y, _) = self.ir.node(operand)?;
                    let narrow_division = op == Op::Div
                        && fact.dims.is_empty()
                        && self.division_ty(x, y)? == EvalTy::F32;
                    if fact.leaf == Leaf::Float && self.is_reactive(operand)? && !narrow_division {
                        let (reads_host, reads_derived) = self.reactive_reads(operand)?;
                        host |= reads_host;
                        derived |= reads_derived;
                        argument |= !reads_host && !reads_derived;
                    }
                    literal |= self.own_float(operand)? == EvalTy::F64;
                }
                if host || literal && (!argument || derived) {
                    EvalTy::F64
                } else {
                    EvalTy::F32
                }
            }
            _ => EvalTy::Exact,
        })
    }

    fn visit(&mut self, node: u32, ty: EvalTy, steps: &mut Vec<Step>) -> Result<(), Internal> {
        if self.unbound_nodes.contains(&node) {
            steps.push(Step::Runtime(0));
            return Ok(());
        }
        let (op, a, b, _) = self.ir.node(node)?;
        let deferred = matches!(ty, EvalTy::DefaultF32 | EvalTy::DefaultF64).then_some(ty);
        let ty = if deferred.is_some() {
            evaluated_in(ty)
        } else {
            ty
        };
        steps.push(Step::Apply(node, ty));
        let flows = !self.unflowed.remove(&node);
        if matches!(op, Op::Name | Op::Field) && !flows {
            self.unflowed.insert(node);
        }
        self.mark_unused_reads(op, node)?;
        self.mark_unexpected_reads(op, node)?;
        let float = matches!(ty, EvalTy::F32 | EvalTy::F64);
        let (mut integer_flow, mut picked, mut float_pair) = (false, false, false);
        let operands: Vec<(u32, EvalTy)> = match op {
            Op::Neg
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Rem
            | Op::Pow
            | Op::Div
            | Op::IntDiv
            | Op::Lt
            | Op::Le
            | Op::Gt
            | Op::Ge
            | Op::Is
            | Op::IsNot
                if self.refused_by_arrays(op, node)? =>
            {
                self.operands(node)?
                    .into_iter()
                    .map(|at| (at, EvalTy::Exact))
                    .collect()
            }
            Op::Neg | Op::Add | Op::Sub | Op::Mul | Op::Rem | Op::Pow => self
                .operands(node)?
                .into_iter()
                .map(|at| (at, deferred.unwrap_or(ty)))
                .collect(),
            Op::Div if deferred.is_some() => {
                let default = deferred.unwrap_or(ty);
                for operand in [b, a] {
                    self.unflowed.insert(operand);
                    let fact = self.fact(operand)?;
                    if fact.fixed.is_none() && fact.dims.is_empty() && fact.leaf == Leaf::Int {
                        steps.push(Step::Float(operand, ty));
                        steps.push(Step::Visit(operand, EvalTy::Exact));
                    } else {
                        steps.push(Step::Visit(operand, default));
                    }
                }
                return Ok(());
            }
            Op::Div if !float => {
                let defaulted = self.fact(a)?.own().is_none() && self.fact(b)?.own().is_none();
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
                self.defer_compared_fit(a, b, pair)?;
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
                float_pair = matches!(pair, EvalTy::F32 | EvalTy::F64)
                    && combined(self.fact(a)?.own(), self.fact(b)?.own(), true).is_none()
                    && !self.is_reactive(node)?;
                vec![(a, pair), (b, pair)]
            }
            Op::And | Op::Or => vec![(a, EvalTy::Bool), (b, EvalTy::Bool)],
            Op::Not => vec![(a, EvalTy::Bool)],
            Op::Call if self.math_name(node)?.is_some_and(super::calls::float_math) => {
                let target = match ty {
                    EvalTy::F32 | EvalTy::F64 => ty,
                    _ => match self.fact(node)?.own() {
                        Some(EvalTy::F64) => EvalTy::F64,
                        _ => EvalTy::F32,
                    },
                };
                for arg in self.arguments(node)?.into_iter().rev() {
                    self.unflowed.insert(arg);
                    let fact = self.fact(arg)?;
                    match (fact.fixed, fact.read) {
                        _ if fact.leaf != Leaf::Int
                            || !fact.dims.is_empty()
                            || deferred.is_none() && matches!(ty, EvalTy::F32 | EvalTy::F64) =>
                        {
                            steps.push(Step::Visit(arg, target));
                        }
                        (None, None) => {
                            steps.push(Step::Float(arg, target));
                            steps.push(Step::Visit(arg, EvalTy::Exact));
                        }
                        (Some(own), _) => {
                            steps.push(Step::Widen(target));
                            steps.push(Step::Visit(arg, own));
                        }
                        (None, Some(_)) => steps.push(Step::Visit(arg, target)),
                    }
                }
                return Ok(());
            }
            Op::Call if self.math_name(node)? == Some("rand") => {
                for arg in self.arguments(node)?.into_iter().rev() {
                    self.unflowed.insert(arg);
                    steps.push(Step::Fit(arg, EvalTy::I64));
                    steps.push(Step::Visit(arg, EvalTy::Exact));
                }
                return Ok(());
            }
            Op::NamedCall
                if self.math_named(node)?.is_some() || self.native_named(node)?.is_some() =>
            {
                Vec::new()
            }
            Op::Call
                if self.math_call(node)?
                    || self.math_miscount(node)?.is_some()
                    || self.native_miscount(node)?.is_some() =>
            {
                self.arguments(node)?
                    .into_iter()
                    .map(|arg| (arg, ty))
                    .collect()
            }
            Op::Call | Op::NamedCall => {
                steps.pop();
                steps.push(Step::Enter(node, ty));
                let mut operands = Vec::new();
                for arg in self.arguments(node)? {
                    let arg_ty = match self.fact(arg)?.fixed {
                        Some(_) => self.own_ty(arg)?.unwrap_or(EvalTy::Exact),
                        None if self.flexible_integer(arg)? => flexible(ty),
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
                let loose = operand.own().is_none()
                    && operand.dims.is_empty()
                    && matches!(own, EvalTy::F32 | EvalTy::F64)
                    && !self.is_reactive(a)?;
                let into_float = target.filter(|target| is_fixed_float(*target));
                let read_ty = match into_float {
                    Some(target) if loose => target,
                    Some(_) => own,
                    None if loose && self.ir.node(a)?.0 == Op::Div => EvalTy::Exact,
                    None if loose && own == EvalTy::F64 => EvalTy::DefaultF64,
                    None if loose => EvalTy::DefaultF32,
                    None => own,
                };
                vec![(a, read_ty)]
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
            Op::Past if self.is_own_history(node) || matches!(self.bound(a)?, DeclRef::Let(_)) => {
                Vec::new()
            }
            Op::Past => vec![(a, ty)],
            Op::Slice => {
                let mut operands = vec![(a, ty)];
                for bound in self.bounds(node)? {
                    if bound != NONE {
                        operands.push((bound, self.integral(bound)?));
                    }
                }
                operands
            }
            Op::Field if self.expr_members.contains(&node) && self.measures_a_literal(node)? => {
                Vec::new()
            }
            Op::Field if self.expr_members.contains(&node) => {
                vec![(a, EvalTy::Exact)]
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
                        (_, None) if fact.leaf == Leaf::Float => self.own_float(rec.iterable.0)?,
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
                Op::Neg
                | Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Rem
                | Op::Pow
                | Op::Div
                | Op::Array
                | Op::Past => true,
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
            if runtime
                && matches!(operand_ty, EvalTy::F32 | EvalTy::F64)
                && self.loose_float(operand)?
            {
                steps.push(Step::FloatFit(operand, operand_ty));
                steps.push(Step::Visit(operand, operand_ty));
                continue;
            }
            if compared
                && matches!(operand_ty, EvalTy::F32 | EvalTy::F64)
                && self.loose_float(operand)?
            {
                if float_pair {
                    steps.push(Step::Widen(EvalTy::F64));
                    steps.push(Step::FloatChoose(operand));
                    steps.push(Step::Visit(operand, EvalTy::DefaultF64));
                } else {
                    steps.push(Step::FloatFit(operand, operand_ty));
                    steps.push(Step::Visit(operand, operand_ty));
                }
                continue;
            }
            if float_pair {
                steps.push(Step::Widen(EvalTy::F64));
            }
            if compared && !reactive && !self.owning && self.fact(operand)?.own().is_some() {
                steps.push(Step::Owning(false));
                steps.push(Step::Widen(operand_ty));
                steps.push(Step::Visit(
                    operand,
                    self.own_ty(operand)?.unwrap_or(operand_ty),
                ));
                steps.push(Step::Owning(true));
                continue;
            }
            let apart = (reactive && stopped && !self.is_reactive(operand)?) || compared;
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

    fn read_in_env(&mut self, read: &super::attr_read::AttrRead) -> Stage<(CoreValue, EvalTy)> {
        let env = self.read_env(read)?;
        let open = (read.root, env, self.inst);
        if !self.open_values.insert(open) {
            return Err(Internal::missed("an open declaration outside every cycle"));
        }
        let saved = self.enter_env(env);
        let chain = std::mem::take(&mut self.eval_chain);
        let outcome = self.read_value(read);
        self.eval_chain = chain;
        self.leave_env(saved);
        self.open_values.remove(&open);
        outcome
    }

    fn read_value(&mut self, read: &super::attr_read::AttrRead) -> Stage<(CoreValue, EvalTy)> {
        let read_ty = match self.fact(read.root)?.fixed {
            Some(own) if widens(own, read.ty) => evaluated_in(read.ty),
            _ => flexible(read.ty),
        };
        Ok(self
            .evaluate(read.root, read_ty)?
            .map(|value| (value, read_ty)))
    }

    pub fn evaluate(&mut self, root: u32, ty: EvalTy) -> Stage<CoreValue> {
        let inst = self.inst;
        let (depth, chain) = (self.eval_chain.len(), self.eval_chain.id());
        let from = self.init_reads.len();
        let outcome = self.evaluate_frames(root, ty);
        self.flush_init_reads(from)?;
        self.inst = inst;
        self.chain_arena.restore(&mut self.eval_chain, depth, chain);
        outcome
    }

    fn enter_let_frame(&mut self, outer_chains: &mut Vec<CallChain>) {
        if !self.eval_chain.is_empty() {
            self.unmemoized();
        }
        outer_chains.push(std::mem::take(&mut self.eval_chain));
    }

    fn evaluate_frames(&mut self, root: u32, ty: EvalTy) -> Stage<CoreValue> {
        let ty = evaluated_in(ty);
        let mut outer_chains: Vec<CallChain> = Vec::new();
        let mut open = super::column::ColumnSet::new();
        let unit = match self.default_scale {
            Some((scaled, family, unit, _)) if scaled == root => {
                Some(self.frame_unit(root, self.inst, family, unit)?)
            }
            Some(_) | None => None,
        };
        let mut scaled = super::column::ColumnSet::new();
        let mut merged = super::column::TripleSet::default();
        merge_reach(&mut scaled, &mut merged, unit.as_ref());
        let mut frames: Vec<Frame> = vec![Frame {
            owner: None,
            ty,
            steps: vec![Step::Visit(root, ty)],
            values: Vec::new(),
            call: None,
            inst: self.inst,
            fold: None,
            unit,
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
                let frame_unit = frame.unit.as_ref().map(|unit| (unit.family, unit.unit));
                let called = frame.call.is_some();
                let (call_node, body) = (frame.call.as_ref().map(|(call, _)| *call), frame.inst);
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
                        if let Some((family, unit)) = frame_unit {
                            self.unit_let_values
                                .insert((at, class(frame_ty), family, unit), value.clone());
                        } else if let Some(slot) = self
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
                        if let Some(call) = call_node
                            && matches!(self.ir.node(call)?.0, Op::Call | Op::NamedCall)
                        {
                            let root = self.body_value(body)?;
                            self.return_init_read(call, root);
                        }
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
                    if !self.family_faults.is_empty() && self.family_faults.contains(&node) {
                        if let Some(frame) = frames.last_mut() {
                            frame.values.push(Value::Error);
                        }
                        continue;
                    }
                    if let Some(frame) = frames.last_mut()
                        && frame.unit.as_ref().is_some()
                        && scaled.contains(&node)
                    {
                        frame.steps.push(Step::Scale(node));
                    }
                    if self.fitting
                        && matches!(op, Op::Name | Op::Field)
                        && self.is_reactive(node)?
                    {
                        if let Some(frame) = frames.last_mut() {
                            frame.values.push(Value::Error);
                        }
                        continue;
                    }
                    if op == Op::Field
                        && (self.array_members.contains_key(&node)
                            || self.expr_members.contains(&node)
                            || self.runtime_members.contains(&node)
                            || self.capacity_members.contains(&node)
                            || self.bare_cell_read(node)?.is_some())
                    {
                        let Some(frame) = frames.last_mut() else {
                            return Err(Internal::missed("evaluation frame"));
                        };
                        self.visit(node, node_ty, &mut frame.steps)?;
                        continue;
                    }
                    if op == Op::Field
                        && let Some(row) = self.path_row(node)
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
                            Some(read) if !read.cyclic && self.read_env(&read)? != NONE => {
                                if self.scaled_reads.contains_key(&self.read_at(node)?) {
                                    return Ok(Err(super::declined::Declined(
                                        super::reason::Reason::UnitAttribute,
                                    )));
                                }
                                match self.read_in_env(&read)? {
                                    Ok((value, read_ty)) => Some((value, NONE, read_ty)),
                                    Err(declined) => return Ok(Err(declined)),
                                }
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
                                    if open.contains(&owner) {
                                        return Err(Internal::missed(
                                            "an open declaration outside every cycle",
                                        ));
                                    }
                                    open.insert(owner);
                                    let unit = match self.scaled_reads.get(&at).copied() {
                                        Some((family, unit)) => {
                                            Some(self.frame_unit(root, NONE, family, unit)?)
                                        }
                                        None => None,
                                    };
                                    merge_reach(&mut scaled, &mut merged, unit.as_ref());
                                    frames.push(Frame {
                                        owner: Some(owner),
                                        ty: read_ty,
                                        steps: vec![Step::Visit(root, read_ty)],
                                        values: Vec::new(),
                                        call: None,
                                        inst: NONE,
                                        fold: None,
                                        unit,
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
                                let called = frames
                                    .iter()
                                    .rev()
                                    .find(owns)
                                    .and_then(|open| open.call.as_ref())
                                    .map(|(call, _)| *call);
                                if let Some(call) = called
                                    && matches!(self.ir.node(call)?.0, Op::Call | Op::NamedCall)
                                    && let Some(argument) =
                                        self.arguments(call)?.get(position as usize).copied()
                                {
                                    self.bind_init_read(node, argument, node_ty);
                                }
                                let argued = called.or_else(|| {
                                    self.own_calls
                                        .iter()
                                        .rev()
                                        .find(|(_, inst, _)| {
                                            self.bodies
                                                .get(*inst as usize)
                                                .is_some_and(|body| body.function == owner)
                                        })
                                        .map(|(call, _, _)| *call)
                                });
                                let value = match argued {
                                    Some(call) => {
                                        self.parameter_fit(call, position, node, node_ty, value)?
                                    }
                                    None => value,
                                };
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
                            | DeclRef::Fn(_)
                            | DeclRef::Pattern(_) => {}
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
                        let let_unit = match frame.unit.as_ref() {
                            Some(unit) if unit.reach.entered.contains(&node) => {
                                let (family, unit) = (unit.family, unit.unit);
                                Some(self.frame_unit(self.let_root(at)?, NONE, family, unit)?)
                            }
                            Some(_) | None => None,
                        };
                        let memo_unit = let_unit.as_ref().map(|unit| (unit.family, unit.unit));
                        merge_reach(&mut scaled, &mut merged, let_unit.as_ref());
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
                            if let Some(value) = self.let_memo(at, let_ty, memo_unit) {
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
                                unit: let_unit,
                            });
                            continue;
                        }
                        frame.steps.push(Step::Record(node, false, at, node_ty));
                        let flexible_let = self.fact(self.let_root(at)?)?.fixed.is_none();
                        if flexible_let && (4..=6).contains(&class(node_ty)) {
                            frame.steps.push(Step::Fit(node, node_ty));
                        }
                        if let Some(value) = self.let_memo(at, node_ty, memo_unit) {
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
                        } else if matches!(node_ty, EvalTy::DefaultF32 | EvalTy::DefaultF64) {
                            node_ty
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
                            unit: let_unit,
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
                        Value::Error
                            if self.is_reactive(condition)?
                                || self.fact(condition)?.leaf != Leaf::Placeholder =>
                        {
                            if unflowed {
                                self.unflowed.extend([then, otherwise]);
                            }
                            frame.steps.push(Step::Runtime(2));
                            for arm in [otherwise, then] {
                                if self.compile_time_operand(arm, arm_ty)? {
                                    frame.steps.push(Step::Fit(arm, arm_ty));
                                    frame.steps.push(Step::Visit(arm, EvalTy::Exact));
                                } else {
                                    frame.steps.push(Step::Visit(arm, arm_ty));
                                }
                            }
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
                    if self.is_reactive(fold)?
                        && let Err(declined) = self.annotated_body_fits(fold)?
                    {
                        return Ok(Err(declined));
                    }
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
                    let fold_unit = match frame.unit.as_ref() {
                        Some(unit) if unit.reach.entered.contains(&fold) => {
                            let (family, unit) = (unit.family, unit.unit);
                            Some(self.frame_unit(rec.body, body, family, unit)?)
                        }
                        Some(_) | None => None,
                    };
                    merge_reach(&mut scaled, &mut merged, fold_unit.as_ref());
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
                        unit: fold_unit,
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
                    let value = self.body_value(body)?;
                    let call_unit = match frame.unit.as_ref() {
                        Some(unit) if unit.reach.entered.contains(&call) => {
                            let (family, unit) = (unit.family, unit.unit);
                            Some(self.frame_unit(value, body, family, unit)?)
                        }
                        Some(_) | None => None,
                    };
                    merge_reach(&mut scaled, &mut merged, call_unit.as_ref());
                    let tag = match call_unit.as_ref() {
                        Some(unit) => {
                            let outermost = self.eval_chain.first().copied().unwrap_or(call);
                            let conflicted = !self.unit_conflicted.is_empty();
                            Some((
                                unit.family,
                                unit.unit,
                                if conflicted { outermost } else { NONE },
                            ))
                        }
                        None => None,
                    };
                    let key = self.memo_key(body, call_ty, &args, tag);
                    if let Some(key) = key.as_ref()
                        && let Some(value) = self.memo_hit(key, call)
                    {
                        frame.values.push(value);
                        continue;
                    }
                    if let Some(key) = key {
                        self.begin_recording(key, frames.len() + 1);
                    }
                    self.push_call(call);
                    frames.push(Frame {
                        owner: None,
                        ty: call_ty,
                        steps: vec![Step::Visit(value, call_ty)],
                        values: Vec::new(),
                        call: Some((call, args)),
                        inst: body,
                        fold: None,
                        unit: call_unit,
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
                        let links = self.use_links.iter().map(|(item, _)| *item).collect();
                        CountedWalk::CoreCallChain.step_by(self.eval_chain.len());
                        self.uses
                            .entry((read, class(read_ty), self.eval_chain.to_vec(), links))
                            .or_insert(((element, target), value));
                    }
                }
                Step::Owning(owning) => self.owning = owning,
                Step::FloatChoose(operand) => {
                    if let Some(value) = frame.values.pop() {
                        self.float_choose(operand, value, &mut frame.steps, &mut frame.values)?;
                    }
                }
                Step::FloatFit(operand, fit_ty) => {
                    if let Some(value) = frame.values.pop() {
                        self.float_fit(operand, value, fit_ty, &mut frame.values)?;
                    }
                }
                Step::Scale(node) => {
                    let Some((family, unit)) =
                        frame.unit.as_ref().map(|unit| (unit.family, unit.unit))
                    else {
                        continue;
                    };
                    let read = frame
                        .owner
                        .and_then(|owner| {
                            owner.checked_sub(u32::try_from(self.ir.lets.len()).ok()?)
                        })
                        .filter(|at| self.scaled_reads.contains_key(at))
                        .and_then(|at| self.attr_reads.get(at as usize))
                        .map(|read| (read.root, family, unit, read.ty));
                    let value = frame.values.pop().ok_or(Internal::missed("value stack"))?;
                    let outer = read.map(|read| self.default_scale.replace(read));
                    let value = if self.scale_conflicted(node)? {
                        value
                    } else {
                        self.in_default_unit(node, value, family, unit)?
                    };
                    let value = if self.fixed_root_outside(node, &value)? {
                        self.saturated_root(node, value)?
                    } else {
                        value
                    };
                    if let Some(outer) = outer {
                        self.default_scale = outer;
                    }
                    if let Some(frame) = frames.last_mut() {
                        frame.values.push(value);
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

    fn measures_a_literal(&self, node: u32) -> Result<bool, Internal> {
        if self.is_bytes_member(node)? || !self.array_members.contains_key(&node) {
            return Ok(false);
        }
        let mut stack = vec![self.ir.node(node)?.1];
        while let Some(at) = stack.pop() {
            let (op, left, right, _) = self.ir.node(at)?;
            if op == Op::Add {
                stack.extend([left, right]);
            } else if op != Op::Array {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn bytes_value(&mut self, receiver: &CoreValue) -> CoreValue {
        let Value::Array(array) = receiver else {
            return Value::Error;
        };
        let count = usize::try_from(self.store.len(*array)).unwrap_or(0);
        let mut bytes = Vec::with_capacity(count);
        for position in 0..count {
            CountedWalk::ValueRun.step();
            let element = self.store.element(*array, position).unwrap_or(Value::Error);
            bytes.push(crate::semantics::operators::convert(element, EvalTy::U8));
        }
        Value::Array(self.store.literal(bytes))
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
                match (self.suffix_ty(node)?, self.unit_literal(node, ty)?) {
                    (_, Some(value)) => value,
                    (Some(own), None) => self.suffixed_integer(node, value, own)?,
                    (None, None) => self.integer_literal(node, value, ty)?,
                }
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
                let own = self.suffix_ty(node)?.unwrap_or(ty);
                match self.unit_literal(node, ty)? {
                    Some(value) => value,
                    None => self.float_literal(node, &literal, own)?,
                }
            }
            Op::Bool => Value::Bool(a == 1),
            Op::Name => match self.bound(node)? {
                DeclRef::Let(at) => self.memo(at, ty).unwrap_or(Value::Error),
                DeclRef::Cell(at) => match self.read_cell(at, node, ty, unflowed)? {
                    Ok(value) => value,
                    Err(declined) => return Ok(Err(declined)),
                },
                DeclRef::None if self.pi.contains(&node) => match ty {
                    EvalTy::F32 => Value::F32(std::f32::consts::PI),
                    EvalTy::F64 => Value::F64(std::f64::consts::PI),
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
                    | EvalTy::Node => Value::Error,
                },
                DeclRef::None if self.is_bare(node) => self.variant_value(node)?,
                DeclRef::Duplicate
                | DeclRef::None
                | DeclRef::Param(_)
                | DeclRef::Fn(_)
                | DeclRef::Pattern(_) => Value::Error,
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
                self.step_init_read((node, a), &position, false);
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
                self.step_init_read((node, a), &offset, true);
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
                let mut bounds = values.iter().skip(1);
                let low = lower.and_then(|(_, end)| bounds.next().map(|at| (at, end)));
                let high = upper.and_then(|(_, end)| bounds.next().map(|at| (at, end)));
                self.slice_init_read((node, a), low, high);
                plan_op(
                    self,
                    Operator::Slice(a, lower, upper),
                    Operands::Many(values),
                )?
            }
            Op::Past => {
                if !self.is_own_history(node) && !matches!(self.bound(a)?, DeclRef::Let(_)) {
                    pop(stack)?;
                    self.pass_init_read(node, a);
                }
                Value::Error
            }
            Op::Stepped => Value::Error,
            Op::Variant => self.variant_value(node)?,
            Op::Field if self.history_capacities.contains_key(&node) => {
                pop(stack)?;
                let (at, floor) = self
                    .history_capacities
                    .get(&node)
                    .copied()
                    .unwrap_or((NONE, 0));
                self.cell_capacity(at)?
                    .map_or(Value::Error, |capacity| Value::Int(capacity.max(floor)))
            }
            Op::Field if self.expr_members.contains(&node) && self.is_bytes_member(node)? => {
                let receiver = pop(stack)?;
                self.bytes_value(&receiver)
            }
            Op::Field if self.expr_members.contains(&node) && self.measures_a_literal(node)? => {
                self.array_members
                    .get(&node)
                    .map_or(Value::Error, |length| Value::Int(*length))
            }
            Op::Field if self.expr_members.contains(&node) => match pop(stack)? {
                Value::Array(array) => Value::Int(i128::from(self.store.len(array))),
                _ => self
                    .array_members
                    .get(&node)
                    .map_or(Value::Error, |length| Value::Int(*length)),
            },
            Op::Field if self.runtime_members.contains(&node) => Value::Error,
            Op::Field if self.capacity_members.contains(&node) => {
                let cell = self.cell_sizes.get(&node).copied().unwrap_or(NONE);
                self.cell_capacity(cell)?.map_or(Value::Error, Value::Int)
            }
            Op::Field if self.array_members.contains_key(&node) => {
                Value::Int(self.array_members.get(&node).copied().unwrap_or(0))
            }
            Op::Field => match self.bare_cell_read(node)? {
                Some(at) => match self.read_cell(at, node, ty, unflowed)? {
                    Ok(value) => value,
                    Err(declined) => return Ok(Err(declined)),
                },
                None => return Err(Internal::missed("a path read of a cell")),
            },
            Op::NamedCall
                if self.math_named(node)?.is_some() || self.native_named(node)?.is_some() =>
            {
                Value::Error
            }
            Op::Call
                if self.math_miscount(node)?.is_some() || self.native_miscount(node)?.is_some() =>
            {
                for _ in self.arguments(node)? {
                    pop(stack)?;
                }
                Value::Error
            }
            Op::Call if self.math_call(node)? => {
                let name = self.math_name(node)?.unwrap_or_default();
                let arity = super::calls::math_arity(name).unwrap_or(1) as usize;
                let mut args = Vec::with_capacity(arity);
                for _ in 0..arity {
                    args.push(pop(stack)?);
                }
                args.reverse();
                match (name, args.as_slice()) {
                    ("abs", [Value::Int(value)]) => crate::semantics::operators::integer(
                        self,
                        node,
                        ty,
                        Some(value.saturating_abs()),
                    )?,
                    ("abs", [other]) => absolute(other.clone()),
                    ("min", [x, y]) => least(x, y, true),
                    ("max", [x, y]) => least(x, y, false),
                    ("clamp", [x, lo, hi]) => least(&least(x, lo, false), hi, true),
                    ("rand", [Value::Int(seed), Value::Int(n)]) => {
                        match (i64::try_from(*seed), i64::try_from(*n)) {
                            (Ok(seed), Ok(n)) => Value::F32(symphonyscript_core::rand(seed, n)),
                            _ => Value::Error,
                        }
                    }
                    ("lerp", [Value::F32(a), Value::F32(b), Value::F32(t)]) => {
                        Value::F32(a + (b - a) * t)
                    }
                    ("lerp", [Value::F64(a), Value::F64(b), Value::F64(t)]) => {
                        Value::F64(a + (b - a) * t)
                    }
                    (name, [Value::F32(x)]) => Value::F32(float_fn32(name, *x)),
                    (name, [Value::F64(x)]) => Value::F64(float_fn64(name, *x)),
                    _ => Value::Error,
                }
            }
            Op::If
            | Op::Str
            | Op::Call
            | Op::NamedCall
            | Op::MethodCall
            | Op::NamedMethodCall
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
        if self.fitting || self.is_cyclic_cell(at) {
            return Ok(Ok(Value::Error));
        }
        if let Some(rec) = self.ir.cells.get(at as usize).copied()
            && rec.rule != NONE
            && let Some(slot) = self
                .rule_reads
                .get_mut(at as usize)
                .and_then(|reads| reads.get_mut(class(ty)))
            && slot.is_none()
            && !self.compared_reads.contains(&node)
        {
            *slot = Some((ty, unflowed));
            self.read_log.push(at);
        }
        let variable = self.cell_capacity_annotation(at)?.is_some();
        if !self.owning
            && (variable || self.cell_annotation_ty(at)?.is_none())
            && int_range(ty).is_some()
            && let Some(rec) = self.ir.cells.get(at as usize).copied()
            && rec.rule == NONE
        {
            let init = match self.evaluate(rec.init, EvalTy::Exact)? {
                Ok(init) => init,
                Err(declined) => return Ok(Err(declined)),
            };
            let own = self.cell_annotation_ty(at)?.and_then(int_range);
            let misses_own = own.is_some_and(|own| self.outside_range(&init, &own));
            match init {
                Value::Int(_) | Value::Array(_) if !misses_own => {
                    self.hold_init_read(node, at, init, ty);
                    if variable {
                        self.whole_init_read();
                    }
                }
                Value::Int(_) | Value::Array(_) => {}
                Value::F32(_)
                | Value::F64(_)
                | Value::Bool(_)
                | Value::Enum(..)
                | Value::Str(_)
                | Value::Node(_)
                | Value::Error => {}
            }
        }
        let side = match class(ty) {
            _ if self.unused_reads.remove(&(self.inst, node)) => None,
            _ if self.unexpected_reads.remove(&(self.inst, node)) => Some(0),
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
        if length == ANY_LENGTH || self.variable_length(base)? {
            if i64::try_from(*value).is_err() {
                self.draw_at(Code::IndexOutOfRange, index, outside_i64(at, *value))?;
            }
            return Ok(());
        }
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

    pub fn own_float(&self, root: u32) -> Result<EvalTy, Internal> {
        let (mut roots, mut seen) = (vec![root], super::column::ColumnSet::new());
        while let Some(root) = roots.pop() {
            for node in self.postorder(root)? {
                let (op, a, _, _) = self.ir.node(node)?;
                if op == Op::Float
                    && let Some(entry) = self.ir.floats.get(a as usize)
                    && entry.value.abs() > f64::from(f32::MAX)
                {
                    return Ok(EvalTy::F64);
                }
                if op == Op::Name
                    && let DeclRef::Let(at) = self.bound(node)?
                    && self.fact(node)?.own().is_none()
                    && seen.insert(at)
                {
                    roots.push(self.let_root(at)?);
                }
            }
        }
        Ok(EvalTy::F32)
    }

    fn arms_unified(&self, choice: u32, own: Option<EvalTy>) -> Result<Option<EvalTy>, Internal> {
        let fact = self.fact(choice)?;
        if !fact.dims.is_empty() {
            return Ok(None);
        }
        if let Some(fixed) = fact.own() {
            return Ok(Some(fixed));
        }
        if fact.leaf != Leaf::Float {
            return Ok(None);
        }
        let (_, _, then, otherwise) = self.ir.node(choice)?;
        let (mut fixed, mut wide) = (false, false);
        for arm in [then, otherwise] {
            if self.fact(arm)?.own().is_some() || self.is_reactive(arm)? {
                fixed = true;
                wide |= matches!(self.own_ty(arm)?, Some(EvalTy::I64 | EvalTy::F64));
            }
        }
        Ok(match (fixed, wide) {
            (false, _) => None,
            (true, true) => Some(EvalTy::F64),
            (true, false) => own,
        })
    }

    fn flexible_integer(&self, arg: u32) -> Result<bool, Internal> {
        let fact = self.fact(arg)?;
        Ok(fact.fixed.is_none()
            && fact.leaf == Leaf::Int
            && fact.dims.is_empty()
            && !self.is_reactive(arg)?)
    }

    fn parameter_fit(
        &mut self,
        call: u32,
        position: u32,
        read: u32,
        read_ty: EvalTy,
        value: CoreValue,
    ) -> Result<CoreValue, Internal> {
        if !matches!(
            self.ir.node(call).map(|node| node.0),
            Ok(Op::Call | Op::NamedCall)
        ) {
            return Ok(value);
        }
        let Some(argument) = self.arguments(call)?.get(position as usize).copied() else {
            return Ok(value);
        };
        if let Value::Int(held) = &value
            && (4..=6).contains(&class(read_ty))
            && int_range(read_ty).is_some_and(|range| !range.contains(held))
            && self.flexible_integer(argument)?
        {
            self.draw_at(
                Code::DoesNotFit,
                read,
                format!("this value does not fit `{}`", ty_name(read_ty)),
            )?;
            return Ok(Value::Error);
        }
        Ok(value)
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
            Leaf::Float => Some(self.float_of(node)?),
            Leaf::Bool => Some(EvalTy::Bool),
            Leaf::Enum(Some(id)) => Some(EvalTy::Enum(id)),
            Leaf::Placeholder | Leaf::Error | Leaf::Enum(None) => None,
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
            Some((array, _)) => {
                let fact = self.fact(array)?;
                matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) && !fact.node
            }
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
        let mut unfixed = super::column::PairColumn::default();
        let mut flowing: super::column::PairColumn<(EvalTy, bool)> = Default::default();
        if let Some(ty) = expected {
            flowing.insert((self.inst, root), (ty, true));
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
                let (flowing_ty, placed) = match flowing.get(&(self.inst, node)).copied() {
                    Some((ty, placed)) => (Some(ty), placed),
                    None => (None, false),
                };
                let flexible_arm = match flowing_ty {
                    Some(ty)
                        if !self.is_reactive(node)?
                            && fact.fixed.is_none()
                            && fact.dims.is_empty()
                            && matches!(fact.leaf, Leaf::Int | Leaf::Float)
                            && (is_fixed_float(ty)
                                || (fact.leaf == Leaf::Int && int_range(ty).is_some())) =>
                    {
                        Some(ty)
                    }
                    Some(_) | None => None,
                };
                let flowed = match flowing_ty {
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
                let evaluated = if let Some(ty) = flexible_arm {
                    flexible(ty)
                } else if let Some(ty) = flowed {
                    ty
                } else if !divided {
                    own
                } else if self.is_reactive(node)? {
                    self.division_ty(a, b)?
                } else {
                    EvalTy::Exact
                };
                let owning = std::mem::replace(&mut self.owning, flowed.is_none() || !placed);
                let outcome = self.evaluate(node, evaluated);
                self.owning = owning;
                let value = match outcome? {
                    Ok(value) => value,
                    Err(declined) => return Ok(Err(declined)),
                };
                if let (Some(ty), Value::Int(held)) = (flexible_arm, &value)
                    && int_range(ty).is_some_and(|range| !range.contains(held))
                {
                    self.draw_at(
                        Code::DoesNotFit,
                        node,
                        format!("this value does not fit `{}`", ty_name(ty)),
                    )?;
                }
                if indexed && fact.dims.is_empty() {
                    self.index_outside_i64(node, &value)?;
                }
                continue;
            }
            match op {
                Op::If => {
                    let unified = self.arms_unified(node, own)?;
                    if let Some(flow) = flowing
                        .get(&(self.inst, node))
                        .copied()
                        .or(unified.map(|ty| (ty, false)))
                    {
                        flowing.insert((self.inst, b), flow);
                        flowing.insert((self.inst, c), flow);
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
                Op::Index | Op::IndexFromEnd
                    if flowing.contains_key(&(self.inst, node))
                        && self.is_kept(node)?
                        && self.is_reactive(a)?
                        && !self.fact(a)?.dims.is_empty() =>
                {
                    let ty = flowing
                        .get(&(self.inst, node))
                        .map_or(EvalTy::Exact, |flow| flow.0);
                    stack.push((b, true));
                    let owning = std::mem::replace(&mut self.owning, false);
                    let outcome = self.evaluate(a, ty);
                    self.owning = owning;
                    if let Err(declined) = outcome? {
                        return Ok(Err(declined));
                    }
                }
                Op::Index | Op::IndexFromEnd => match self.pick_operands(node)? {
                    Some(operands) => {
                        let mut values = Vec::new();
                        for operand in operands {
                            let ty = self.integral(operand)?;
                            match self.known(operand, ty, &mut stack)? {
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
                            None => self.integral(bound)?,
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
                    let body_float = self.body_float(node)?;
                    for arg in self.arguments(node)? {
                        let held = match self.own_ty(arg)? {
                            Some(own) if !self.is_kept(arg)? && !self.is_reactive(arg)? => {
                                self.unflowed.insert(arg);
                                let loose = self.fact(arg)?;
                                let own = if self.flexible_integer(arg)? {
                                    flexible(own)
                                } else if let Some(float) = body_float
                                    && loose.fixed.is_none()
                                    && loose.leaf == Leaf::Float
                                    && loose.dims.is_empty()
                                {
                                    float
                                } else {
                                    own
                                };
                                match self.evaluate(arg, own)? {
                                    Ok(value) => Some(value),
                                    Err(declined) => return Ok(Err(declined)),
                                }
                            }
                            Some(own) if !self.is_kept(arg)? => {
                                match self.bound_in_own(arg, own)? {
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
                        let width = self.call_widths.get(&(self.inst_key(node)?, node)).copied();
                        leaves.push(std::mem::replace(&mut self.inst, body));
                        self.push_call(node);
                        self.own_calls.push((node, body, args));
                        stack.push((NONE, false));
                        stack.push((value, false));
                        if let Some(width) = width
                            && width == EvalTy::F32
                            && self.is_reactive(value)?
                            && self.own_ty(value)? == Some(EvalTy::F64)
                        {
                            stack.pop();
                            let outcome = self.evaluate(value, width);
                            if let Err(declined) = outcome? {
                                return Ok(Err(declined));
                            }
                        }
                    }
                }
                Op::For => {
                    if let Err(declined) = self.annotated_body_fits(node)? {
                        return Ok(Err(declined));
                    }
                    for operand in self.operands(node)?.into_iter().rev() {
                        stack.push((operand, false));
                    }
                }
                Op::Name => {
                    if !self.is_history_target(node)
                        && let DeclRef::Let(at) = self.bound(node)?
                        && !self.cyclic.get(at as usize).copied().unwrap_or(false)
                        && let Some(owned) = self.owned.get_mut(at as usize)
                        && !*owned
                    {
                        *owned = true;
                        self.unmemoized();
                        fits.push(at);
                        let root = self.let_root(at)?;
                        match self.own_ty(root)? {
                            Some(own) if self.local_runtime_let(at)? && !self.is_kept(root)? => {
                                if let Err(declined) = self.bound_in_own(root, own)? {
                                    return Ok(Err(declined));
                                }
                            }
                            Some(_) | None => stack.push((root, false)),
                        }
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
        unfixed: &mut super::column::PairColumn<bool>,
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
        unfixed: &mut super::column::PairColumn<bool>,
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
                    | DeclRef::Fn(_)
                    | DeclRef::Pattern(_) => fact.own().is_none(),
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

    pub fn integer_literal(
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

    pub fn float_literal(
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

impl Core<'_> {
    fn bound_in_own(&mut self, root: u32, own: EvalTy) -> Stage<CoreValue> {
        self.unflowed.insert(root);
        let owning = std::mem::replace(&mut self.owning, false);
        let outcome = self.evaluate(root, own);
        self.owning = owning;
        outcome
    }

    fn local_runtime_let(&self, at: u32) -> Result<bool, Internal> {
        let local = self
            .ir
            .lets
            .get(at as usize)
            .is_some_and(|rec| rec.owner != NONE);
        Ok(local && self.is_reactive(self.let_root(at)?)?)
    }

    fn mark_unexpected_reads(&mut self, op: Op, node: u32) -> Result<(), Internal> {
        if matches!(op, Op::Name | Op::Field) {
            return Ok(());
        }
        let inherited = self.unexpected_reads.remove(&(self.inst, node));
        let marked: Vec<u32> = match op {
            Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot => {
                let mut marked = Vec::new();
                for operand in self.operands(node)? {
                    if self.fact(operand)?.leaf != Leaf::Int {
                        marked.push(operand);
                    }
                }
                marked
            }
            Op::Convert if !self.fact(node)?.fixed.is_some_and(is_fixed_float) => {
                self.operands(node)?
            }
            Op::Call if inherited && self.math_call(node)? => self.operands(node)?,
            Op::Neg
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IntDiv
            | Op::Rem
            | Op::Pow
            | Op::If
            | Op::Array
            | Op::Index
            | Op::IndexFromEnd
            | Op::Slice
                if inherited =>
            {
                self.operands(node)?
            }
            Op::Int
            | Op::Float
            | Op::Bool
            | Op::Str
            | Op::Name
            | Op::Variant
            | Op::To
            | Op::Convert
            | Op::Field
            | Op::Neg
            | Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::IntDiv
            | Op::Rem
            | Op::Pow
            | Op::If
            | Op::Array
            | Op::Index
            | Op::IndexFromEnd
            | Op::Slice
            | Op::Call
            | Op::NamedCall
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Past
            | Op::Stepped
            | Op::Not
            | Op::And
            | Op::Or
            | Op::Match
            | Op::For
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => Vec::new(),
        };
        for operand in marked {
            self.unexpected_reads.insert((self.inst, operand));
        }
        Ok(())
    }

    fn mark_unused_reads(&mut self, op: Op, node: u32) -> Result<(), Internal> {
        if matches!(op, Op::Name | Op::Field) {
            return Ok(());
        }
        let refused = matches!(
            op,
            Op::Neg
                | Op::Add
                | Op::Sub
                | Op::Mul
                | Op::Rem
                | Op::Pow
                | Op::Div
                | Op::IntDiv
                | Op::Lt
                | Op::Le
                | Op::Gt
                | Op::Ge
                | Op::Is
                | Op::IsNot
        ) && self.refused_by_arrays(op, node)?;
        if self.unused_reads.remove(&(self.inst, node)) || refused {
            for operand in self.operands(node)? {
                self.unused_reads.insert((self.inst, operand));
            }
        }
        Ok(())
    }

    fn refused_by_arrays(&self, op: Op, node: u32) -> Result<bool, Internal> {
        let mut arrays = 0;
        for operand in self.operands(node)? {
            if !self.fact(operand)?.dims.is_empty() {
                arrays += 1;
            }
        }
        Ok(if op == Op::Add {
            arrays == 1
        } else {
            arrays > 0
        })
    }
}

fn absolute(value: CoreValue) -> CoreValue {
    match value {
        Value::F32(value) => Value::F32(value.abs()),
        Value::F64(value) => Value::F64(value.abs()),
        other => other,
    }
}

fn least(x: &CoreValue, y: &CoreValue, min: bool) -> CoreValue {
    let pick_x = |x_first: bool| if x_first == min { x.clone() } else { y.clone() };
    match (x, y) {
        (Value::Int(a), Value::Int(b)) => pick_x(a <= b),
        (Value::F32(a), Value::F32(b)) => {
            Value::F32(float_least(f64::from(*a), f64::from(*b), min) as f32)
        }
        (Value::F64(a), Value::F64(b)) => Value::F64(float_least(*a, *b, min)),
        _ => Value::Error,
    }
}

fn float_least(a: f64, b: f64, min: bool) -> f64 {
    if a.is_nan() {
        return b;
    }
    if b.is_nan() {
        return a;
    }
    if a == b {
        let negative = a.is_sign_negative() || b.is_sign_negative();
        let positive = a.is_sign_positive() || b.is_sign_positive();
        return match (min, negative, positive) {
            (true, true, _) => -a.abs(),
            (false, _, true) => a.abs(),
            _ => a,
        };
    }
    if (a < b) == min { a } else { b }
}

fn float_fn32(name: &str, x: f32) -> f32 {
    match name {
        "abs" => x.abs(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "sqrt" => x.sqrt(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "exp" => x.exp(),
        "log" => x.ln(),
        _ => f32::NAN,
    }
}

fn float_fn64(name: &str, x: f64) -> f64 {
    match name {
        "abs" => x.abs(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "sqrt" => x.sqrt(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "exp" => x.exp(),
        "log" => x.ln(),
        _ => f64::NAN,
    }
}

impl Core<'_> {
    fn body_float(&mut self, call: u32) -> Result<Option<EvalTy>, Internal> {
        if let Some(width) = self.call_widths.get(&(self.inst_key(call)?, call)) {
            return Ok(Some(*width));
        }
        let Some(body) = self.body_of_call(call) else {
            return Ok(None);
        };
        let value = self.body_value(body)?;
        let outer = std::mem::replace(&mut self.inst, body);
        let own = self.own_ty(value);
        self.inst = outer;
        Ok(own?.filter(|ty| matches!(ty, EvalTy::F32 | EvalTy::F64)))
    }
}

impl Core<'_> {
    fn float_of(&self, node: u32) -> Result<EvalTy, Internal> {
        if matches!(self.ir.node(node)?.0, Op::Call | Op::NamedCall)
            && let Some(width) = self.call_widths.get(&(self.inst_key(node)?, node))
        {
            return Ok(*width);
        }
        self.own_float(node)
    }

    pub fn reactive_width(&self, value: u32) -> Result<EvalTy, Internal> {
        if let Some(own) = self.fact(value)?.own() {
            return Ok(own);
        }
        let (host, derived) = self.reactive_reads(value)?;
        let argument = !host && !derived;
        let literal = self.own_float(value)? == EvalTy::F64;
        Ok(if host || literal && (!argument || derived) {
            EvalTy::F64
        } else {
            EvalTy::F32
        })
    }
}
