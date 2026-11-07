use super::core::{Core, Stage};
use super::decl_ref::DeclRef;
use super::declined::Declined;
use super::fact::{Fact, reads};
use super::folds::{ANY_LENGTH, dims_agree};
use super::leaf::Leaf;
use super::need::Need;
use super::reason::Reason;
use super::width::{combined, is_fixed_float, target_ty};
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::counted_walk::CountedWalk;
use crate::semantics::ir::flat::{NONE, Op, binary_of};
use crate::semantics::operators::{EvalTy, Value};
use symphonyscript_core::BinaryOp;

pub const OPERAND: &str = "operators and conversions apply to single values, not arrays";
const SINGLE_OPERANDS: &str =
    "only `+` takes arrays, which it joins; every other operator takes single values";
pub(super) const JOIN_SINGLE: &str = "`+` joins two arrays, and one of these is a single value";
const RAGGED: &str = "the elements of an array are arrays of one length, or all single values";

const NEW: u8 = 0;
const OPEN: u8 = 1;
const DONE: u8 = 2;

impl Core<'_> {
    pub fn operands(&self, node: u32) -> Result<Vec<u32>, Internal> {
        let mut operands = Vec::new();
        self.push_operands(node, &mut operands)?;
        Ok(operands)
    }

    pub fn push_operands(&self, node: u32, out: &mut Vec<u32>) -> Result<(), Internal> {
        let (op, a, b, c) = self.ir.node(node)?;
        match op {
            Op::Neg | Op::Not | Op::Convert => out.push(a),
            Op::Add
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
            | Op::Index
            | Op::IndexFromEnd => out.extend([a, b]),
            Op::Slice => {
                out.push(a);
                for at in b..b + 2 {
                    let bound = self.kid(at)?;
                    if bound != NONE {
                        out.push(bound);
                    }
                }
            }
            Op::If => out.extend([a, b, c]),
            Op::Past => {
                out.push(a);
                out.extend([b, c].into_iter().filter(|at| *at != NONE));
            }
            Op::Stepped => out.push(a),
            Op::Call | Op::NamedCall => out.extend(self.arguments(node)?),
            Op::For => out.extend(self.fold_operands(node)?),
            Op::Array => {
                for at in a..a + b {
                    out.push(self.kid(at)?);
                }
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
            | Op::Match
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => {}
        }
        Ok(())
    }

    pub fn postorder(&self, root: u32) -> Result<Vec<u32>, Internal> {
        let mut order = Vec::new();
        let mut stack = vec![(root, false)];
        let mut kids = Vec::new();
        while let Some((node, visited)) = stack.pop() {
            if visited {
                order.push(node);
                continue;
            }
            stack.push((node, true));
            kids.clear();
            self.push_operands(node, &mut kids)?;
            stack.extend(kids.iter().rev().map(|kid| (*kid, false)));
        }
        Ok(order)
    }

    pub fn let_root(&self, at: u32) -> Result<u32, Internal> {
        Ok(self
            .ir
            .lets
            .get(at as usize)
            .ok_or(Internal::missed("IR let"))?
            .value)
    }

    fn history_lets(&self, root: u32) -> Result<Vec<u32>, Internal> {
        let mut lets = Vec::new();
        for node in self.postorder(root)? {
            let (op, a, _, _) = self.ir.node(node)?;
            if op == Op::Past
                && let DeclRef::Let(at) = self.bound(a)?
                && self
                    .ir
                    .lets
                    .get(at as usize)
                    .is_some_and(|rec| rec.owner == NONE)
                && self.annotation_ty(at)?.is_none()
            {
                lets.push(at);
            }
        }
        Ok(lets)
    }

    fn read_lets(&self, root: u32) -> Result<Vec<u32>, Internal> {
        let mut reads = Vec::new();
        for node in self.postorder(root)? {
            if self.is_history_target(node) {
                continue;
            }
            if let DeclRef::Let(at) = self.bound(node)?
                && self
                    .ir
                    .lets
                    .get(at as usize)
                    .is_some_and(|rec| rec.owner == NONE)
            {
                reads.push(at);
            }
        }
        Ok(reads)
    }

    pub fn bound(&self, node: u32) -> Result<DeclRef, Internal> {
        let local = self.local(node)?;
        Ok(self.bind.get(local).copied().unwrap_or(DeclRef::None))
    }

    pub fn ensure_typed(&mut self, root: u32) -> Stage<Fact> {
        let mut stack: Vec<(u32, bool)> = Vec::new();
        for at in self.history_lets(root)? {
            if self.let_state.get(at as usize) == Some(&NEW) {
                stack.push((at, false));
            }
        }
        for at in self.read_lets(root)?.into_iter().rev() {
            stack.push((at, false));
        }
        while let Some((at, expanded)) = stack.pop() {
            let cyclic = self.cyclic.get(at as usize).copied().unwrap_or(false);
            let state = self.let_state.get(at as usize).copied().unwrap_or(DONE);
            if cyclic || state == DONE {
                continue;
            }
            if expanded {
                let top = self.typing_bodies > 0
                    && self.inst != NONE
                    && self
                        .ir
                        .lets
                        .get(at as usize)
                        .is_some_and(|rec| rec.owner == NONE);
                let outer = if top {
                    std::mem::replace(&mut self.inst, NONE)
                } else {
                    self.inst
                };
                let mark = self.diagnostics.len();
                let typed = self.type_root(self.let_root(at)?);
                let typed = match typed {
                    Ok(Ok(())) if top => self
                        .check_annotation(at)
                        .map(|checked| checked.map(|()| true)),
                    Ok(Ok(())) => Ok(Ok(false)),
                    Ok(Err(declined)) => Ok(Err(declined)),
                    Err(internal) => Err(internal),
                };
                if top {
                    self.inst = outer;
                    let drawn = self.diagnostics.split_off(mark);
                    self.top_drawn.extend(drawn);
                }
                let checked = match typed? {
                    Ok(checked) => checked,
                    Err(declined) => {
                        if declined == Declined(Reason::Pending) {
                            let open = stack
                                .iter()
                                .filter(|(_, opened)| *opened)
                                .map(|(at, _)| *at);
                            for reopened in open.chain([at]).collect::<Vec<u32>>() {
                                if let Some(slot) = self.let_state.get_mut(reopened as usize) {
                                    *slot = NEW;
                                }
                            }
                        }
                        return Ok(Err(declined));
                    }
                };
                if !checked && let Err(declined) = self.check_annotation(at)? {
                    return Ok(Err(declined));
                }
                if let Some(slot) = self.let_state.get_mut(at as usize) {
                    *slot = DONE;
                }
                continue;
            }
            if state == OPEN {
                return Err(Internal::missed("an open declaration outside every cycle"));
            }
            if let Some(slot) = self.let_state.get_mut(at as usize) {
                *slot = OPEN;
            }
            stack.push((at, true));
            let root = self.let_root(at)?;
            for read in self
                .history_lets(root)?
                .into_iter()
                .chain(self.read_lets(root)?.into_iter().rev())
            {
                if self.let_state.get(read as usize) == Some(&NEW) {
                    stack.push((read, false));
                }
            }
        }
        if let Err(declined) = self.type_root(root)? {
            return Ok(Err(declined));
        }
        Ok(Ok(self.fact(root)?))
    }

    pub fn type_root(&mut self, root: u32) -> Stage<()> {
        if self.driving {
            return self.type_nodes(root);
        }
        self.driving = true;
        let outcome = self.drive(root);
        self.driving = false;
        self.need = None;
        outcome
    }

    fn drive(&mut self, root: u32) -> Stage<()> {
        let mut jobs = vec![Need::Local(NONE, root, self.inst)];
        while let Some(job) = jobs.last().copied() {
            let outcome = match job {
                Need::Body(body) => self.type_body(body)?,
                Need::Local(_, root, inst) => {
                    let outer = std::mem::replace(&mut self.inst, inst);
                    let outcome = self.type_nodes(root);
                    self.inst = outer;
                    outcome?
                }
            };
            match outcome {
                Ok(()) => {
                    jobs.pop();
                    if let Need::Local(at, _, inst) = job {
                        self.typing_locals.remove(&(inst, at));
                    }
                }
                Err(Declined(Reason::Pending)) => {
                    let need = self.need.take().ok_or(Internal::missed("a pending job"))?;
                    if let Need::Local(at, _, inst) = need
                        && !self.typing_locals.insert((inst, at))
                    {
                        return Ok(Err(Declined(Reason::LocalCycle)));
                    }
                    jobs.push(need);
                }
                Err(declined) => return Ok(Err(declined)),
            }
        }
        Ok(Ok(()))
    }

    pub fn pending<T>(&mut self, need: Need) -> Stage<T> {
        self.need = Some(need);
        Ok(Err(Declined(Reason::Pending)))
    }

    fn type_nodes(&mut self, root: u32) -> Stage<()> {
        if self.has_fact(root)? {
            return Ok(Ok(()));
        }
        let mut references = false;
        for node in self.postorder(root)? {
            if self.has_fact(node)? {
                continue;
            }
            CountedWalk::SigNode.step();
            self.typing_now += 1;
            let outcome = self.node_fact(node);
            self.typing_now -= 1;
            let mut fact = match outcome? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            };
            if let Err(declined) = self.node_family(node, &mut fact)? {
                return Ok(Err(declined));
            }
            if let Err(declined) = self.enum_gate(node, &fact)? {
                return Ok(Err(declined));
            }
            let mut reactive = self.node_reactive(node)?;
            if fact.node
                && reactive
                && self.in_body(node)?.is_some()
                && self.reactive_local_reference(node)?
            {
                fact = Fact::PLACEHOLDER;
                reactive = false;
            }
            references |= fact.node;
            self.set_fact(node, fact, reactive)?;
        }
        if references && !self.holds_references(root) && !self.placed_root(root)? {
            return Ok(Err(Declined(Reason::NameOfInstance)));
        }
        Ok(Ok(()))
    }

    fn node_fact(&mut self, node: u32) -> Stage<Fact> {
        let (op, a, b, c) = self.ir.node(node)?;
        if self.unbound_nodes.contains(&node) {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        if self.holds_a_missing_operand(node)? {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        match self.reference_fact(node)? {
            Ok(Some(fact)) => return Ok(Ok(fact)),
            Ok(None) => {}
            Err(declined) => return Ok(Err(declined)),
        }
        Ok(Ok(match op {
            Op::Int => self.literal_fact(node, Leaf::Int)?,
            Op::Float => self.literal_fact(node, Leaf::Float)?,
            Op::Bool => Fact::scalar(Leaf::Bool),
            Op::Name if self.is_history_target(node) => self.history_target_fact(node)?,
            Op::Name if self.not_values.contains_key(&node) => {
                let summary = self.not_values.get(&node).cloned().unwrap_or_default();
                self.draw_at(Code::NotAValue, node, summary)?;
                self.not_values_drawn.insert(node);
                Fact::PLACEHOLDER
            }
            Op::Name => match self.bound(node)? {
                DeclRef::Let(at) => {
                    let root = self.let_root(at)?;
                    let local = self
                        .ir
                        .lets
                        .get(at as usize)
                        .is_some_and(|rec| rec.owner != NONE);
                    let cyclic = self.cyclic.get(at as usize).copied().unwrap_or(false);
                    if local && !cyclic && !self.has_fact(root)? {
                        let inst = self.inst_key(root)?;
                        if self.typing_locals.contains(&(inst, at)) {
                            return Ok(Err(Declined(Reason::LocalCycle)));
                        }
                        return self.pending(Need::Local(at, root, inst));
                    }
                    if self.cyclic.get(at as usize).copied().unwrap_or(false) {
                        Fact::PLACEHOLDER
                    } else {
                        self.let_fact(at)?
                    }
                }
                DeclRef::Cell(at) if self.derived(at) && !self.has_fact(self.cell_root(at)?)? => {
                    Fact::PLACEHOLDER
                }
                DeclRef::Cell(at) => self.cell_read_fact(at)?,
                DeclRef::Param(position) => self.param_fact(node, position)?.0,
                DeclRef::Fn(_) if self.function_values.contains(&node) => Fact::PLACEHOLDER,
                DeclRef::Fn(_) | DeclRef::Pattern(_) => {
                    return Ok(Err(Declined(Reason::FunctionValue)));
                }
                DeclRef::None if self.pi.contains(&node) => Fact::scalar(Leaf::Float),
                DeclRef::None if self.untyped_bare.contains(&node) => Fact::ERROR,
                DeclRef::None if self.is_bare(node) => {
                    Fact::scalar(Leaf::Enum(self.bare_enums.get(&node).copied()))
                }
                DeclRef::Duplicate | DeclRef::None => Fact::PLACEHOLDER,
                DeclRef::Instance(_) => Fact::node_reference(Vec::new()),
            },
            Op::Neg | Op::Not => self.unary_fact(node, op, a)?,
            Op::Add
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
            | Op::IsNot => self.binary_fact(node, op, a, b)?,
            Op::If => match self.choice_fact(node, a, b, c)? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            },
            Op::Array => self.array_fact(node, a, b)?,
            Op::Index | Op::IndexFromEnd => self.index_fact(node, a, b)?,
            Op::Convert => self.convert_fact(node, a, b)?,
            Op::Call | Op::NamedCall
                if self.dot_calls.contains(&node) && self.member_dot_call(node)? =>
            {
                Fact::PLACEHOLDER
            }
            Op::Call | Op::NamedCall
                if self.dot_calls.contains(&node) && !self.dot_call_lands(node)? =>
            {
                return Ok(Err(Declined(Reason::Member)));
            }
            Op::Call | Op::NamedCall => match self.call_fact(node)? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            },
            Op::For => match self.for_fact(node)? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            },
            Op::Slice => match self.slice_fact(node, a, b, c)? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            },
            Op::Past => match self.history_fact(node, a, b, c)? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            },
            Op::Stepped => match self.stepped_fact(node, a)? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            },
            Op::Field if self.array_members.contains_key(&node) => Fact::scalar(Leaf::Int),
            Op::Field if self.cell_sizes.contains_key(&node) => {
                let at = self.cell_sizes.get(&node).copied().unwrap_or(NONE);
                match self.cell_size_fact(node, at)? {
                    Ok(fact) => fact,
                    Err(declined) => return Ok(Err(declined)),
                }
            }
            Op::Field => match self.path_read(node)?.cloned() {
                Some(read) if read.cyclic => Fact::PLACEHOLDER,
                Some(read) => {
                    if read.root != NONE
                        && let Err(declined) = self.typed_in_env(&read)?
                    {
                        return Ok(Err(declined));
                    }
                    match self.bare_cell_read(node)? {
                        Some(at) => self.cell_read_fact(at)?,
                        None => super::annotation::annotated_fact(read.ty),
                    }
                }
                None => match self.to_choice(node) {
                    Some(ty) => super::annotation::annotated_fact(ty),
                    None => {
                        self.draw_body_path_faults(node);
                        Fact::PLACEHOLDER
                    }
                },
            },
            Op::Variant => match self.variant_fact(node)? {
                Ok(fact) => fact,
                Err(declined) => return Ok(Err(declined)),
            },
            Op::Str
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Match
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar => return Err(Internal::missed("an op the coverage table leaves out")),
            Op::Bad | Op::To => Fact::PLACEHOLDER,
        }))
    }

    fn unary_fact(&mut self, node: u32, op: Op, a: u32) -> Result<Fact, Internal> {
        let operand = self.fact(a)?;
        if operand.leaf == Leaf::Placeholder {
            return Ok(Fact::PLACEHOLDER);
        }
        if !operand.dims.is_empty() {
            if operand.leaf != Leaf::Error {
                self.draw_at(Code::ArrayOperand, node, OPERAND)?;
            }
            return Ok(Fact::PLACEHOLDER);
        }
        let boolean = Fact::scalar(Leaf::Bool);
        match (op, operand.leaf) {
            (Op::Not, Leaf::Bool) => Ok(boolean),
            (Op::Not, Leaf::Error) => self.keep(node, boolean),
            (
                Op::Not,
                Leaf::Placeholder | Leaf::Empty | Leaf::Int | Leaf::Float | Leaf::Enum(_),
            ) => {
                self.draw_at(Code::OperandType, node, "`not` takes a `bool`")?;
                self.keep(node, boolean)
            }
            (Op::Neg, Leaf::Bool | Leaf::Enum(_)) => {
                self.draw_at(Code::OperandType, node, "unary `-` takes a number")?;
                Ok(Fact::ERROR)
            }
            (Op::Neg, Leaf::Empty | Leaf::Int | Leaf::Float) => Ok(Fact::numeric(
                leaf_of(operand.leaf),
                operand.fixed,
                operand.read,
            )),
            (Op::Neg, Leaf::Placeholder | Leaf::Error) => Ok(Fact::ERROR),
            (
                Op::Int
                | Op::Float
                | Op::Bool
                | Op::Str
                | Op::Name
                | Op::Variant
                | Op::To
                | Op::Convert
                | Op::Array
                | Op::Field
                | Op::Index
                | Op::IndexFromEnd
                | Op::Slice
                | Op::Call
                | Op::NamedCall
                | Op::MethodCall
                | Op::NamedMethodCall
                | Op::Past
                | Op::Stepped
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
                | Op::Bad,
                _,
            ) => Err(Internal::missed("a unary operator")),
        }
    }

    pub(super) fn converted_fact(code: u32) -> Result<Fact, Internal> {
        let target = target_ty(code).ok_or(Internal::missed("a conversion target"))?;
        let leaf = if is_fixed_float(target) {
            Leaf::Float
        } else if target == EvalTy::Bool {
            Leaf::Bool
        } else {
            Leaf::Int
        };
        Ok(Fact::fixed(leaf, Some(target)))
    }

    fn convert_fact(&mut self, node: u32, a: u32, code: u32) -> Result<Fact, Internal> {
        let converted = Self::converted_fact(code)?;
        let operand = self.fact(a)?;
        if operand.leaf == Leaf::Placeholder {
            return self.keep_kind(node, converted);
        }
        if !operand.dims.is_empty() {
            if operand.leaf != Leaf::Error {
                self.draw_at(Code::ArrayOperand, node, OPERAND)?;
            }
            return self.keep_kind(node, converted);
        }
        match operand.leaf {
            Leaf::Error => self.keep(node, converted),
            Leaf::Bool | Leaf::Enum(_) => {
                self.draw_at(
                    Code::ConversionType,
                    node,
                    "a `bool` or an enum value is not converted",
                )?;
                self.keep(node, converted)
            }
            Leaf::Placeholder | Leaf::Empty | Leaf::Int | Leaf::Float => Ok(converted),
        }
    }

    fn binary_fact(&mut self, node: u32, op: Op, a: u32, b: u32) -> Result<Fact, Internal> {
        let left = self.fact(a)?;
        let right = self.fact(b)?;
        let boolean = matches!(
            op,
            Op::And | Op::Or | Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot
        );
        if left.leaf == Leaf::Placeholder || right.leaf == Leaf::Placeholder {
            return Ok(Fact::PLACEHOLDER);
        }
        let arrays = (!left.dims.is_empty(), !right.dims.is_empty());
        let open_array = |fact: &Fact| !fact.dims.is_empty() && fact.leaf == Leaf::Error;
        if arrays != (false, false) {
            if op != Op::Add {
                if open_array(&left) || open_array(&right) {
                    return Ok(Fact::PLACEHOLDER);
                }
                self.draw_at(Code::ArrayOperand, node, SINGLE_OPERANDS)?;
                if boolean {
                    return self.keep(node, Fact::scalar(Leaf::Bool));
                }
                return Ok(Fact::PLACEHOLDER);
            }
            if arrays != (true, true) {
                let (single, array) = if arrays.0 {
                    (&right, &left)
                } else {
                    (&left, &right)
                };
                if single.leaf != Leaf::Error && !open_array(array) {
                    self.draw_at(Code::ArrayOperand, node, JOIN_SINGLE)?;
                }
                return Ok(Fact::PLACEHOLDER);
            }
            return self.join_fact(node, &left, &right);
        }
        let (x, y) = (left.leaf, right.leaf);
        if x == Leaf::Error || y == Leaf::Error {
            if boolean {
                return self.keep(node, Fact::scalar(Leaf::Bool));
            }
            return Ok(Fact::ERROR);
        }
        let numeric = |leaf: Leaf| matches!(leaf, Leaf::Int | Leaf::Float | Leaf::Empty);
        let logical = |leaf: Leaf| leaf == Leaf::Bool;
        let kept_bool = Fact::scalar(Leaf::Bool);
        Ok(match op {
            Op::Add | Op::Sub | Op::Mul | Op::Rem | Op::Pow => {
                if !(numeric(x) && numeric(y)) {
                    self.operand_type(node, op)?;
                    return Ok(Fact::ERROR);
                }
                let leaf = numeric_leaf(x, y);
                let float = leaf == Leaf::Float;
                Fact::numeric(
                    leaf,
                    combined(left.fixed, right.fixed, float),
                    reads(left.read, right.read, float),
                )
            }
            Op::Div => {
                if !(numeric(x) && numeric(y)) {
                    self.operand_type(node, op)?;
                    return Ok(Fact::ERROR);
                }
                Fact::numeric(
                    Leaf::Float,
                    combined(left.fixed, right.fixed, true),
                    reads(left.read, right.read, true),
                )
            }
            Op::IntDiv => {
                let integer = |leaf: Leaf| matches!(leaf, Leaf::Int | Leaf::Empty);
                if !(integer(x) && integer(y)) {
                    self.operand_type(node, op)?;
                    return Ok(Fact::ERROR);
                }
                Fact::numeric(
                    Leaf::Int,
                    combined(left.fixed, right.fixed, false),
                    reads(left.read, right.read, false),
                )
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                if !(numeric(x) && numeric(y)) {
                    self.operand_type(node, op)?;
                    return self.keep(node, kept_bool);
                }
                Fact::scalar(Leaf::Bool)
            }
            Op::Is | Op::IsNot => {
                let enums = match (x, y) {
                    (Leaf::Enum(Some(left)), Leaf::Enum(Some(right))) => left == right,
                    (Leaf::Enum(Some(_)), Leaf::Enum(None))
                    | (Leaf::Enum(None), Leaf::Enum(Some(_))) => true,
                    _ => false,
                };
                let comparable = (numeric(x) && numeric(y)) || (logical(x) && logical(y)) || enums;
                if !comparable {
                    self.operand_type(node, op)?;
                    return self.keep(node, kept_bool);
                }
                Fact::scalar(Leaf::Bool)
            }
            Op::And | Op::Or => {
                match (logical(x), logical(y)) {
                    (true, true) => return Ok(Fact::scalar(Leaf::Bool)),
                    (false, false) => self.operand_type(node, op)?,
                    (true, false) => self.draw_at_root(
                        Code::TypeMismatch,
                        b,
                        "this value has the wrong type here; `bool` is expected",
                    )?,
                    (false, true) => self.draw_at_root(
                        Code::TypeMismatch,
                        a,
                        "this value has the wrong type here; `bool` is expected",
                    )?,
                }
                return self.keep(node, kept_bool);
            }
            Op::Int
            | Op::Float
            | Op::Bool
            | Op::Str
            | Op::Name
            | Op::Variant
            | Op::To
            | Op::Convert
            | Op::Array
            | Op::Field
            | Op::Index
            | Op::IndexFromEnd
            | Op::Slice
            | Op::Call
            | Op::NamedCall
            | Op::MethodCall
            | Op::NamedMethodCall
            | Op::Past
            | Op::Stepped
            | Op::Neg
            | Op::Not
            | Op::If
            | Op::Match
            | Op::For
            | Op::TyScalar
            | Op::TyName
            | Op::TyArray
            | Op::TyVar
            | Op::Bad => return Err(Internal::missed("a binary operator")),
        })
    }

    fn operand_type(&mut self, node: u32, op: Op) -> Result<(), Internal> {
        let binary = binary_of(op).ok_or(Internal::missed("a binary operator"))?;
        self.draw_at(Code::OperandType, node, operand_message(binary))
    }

    fn join_fact(&mut self, node: u32, left: &Fact, right: &Fact) -> Result<Fact, Internal> {
        let (Some(left_len), Some(right_len)) = (left.dims.first(), right.dims.first()) else {
            return Err(Internal::missed("array length"));
        };
        let inner = if *left_len == 0 {
            &right.dims
        } else {
            &left.dims
        };
        let (_, a, b, _) = self.ir.node(node)?;
        if *left_len != 0
            && *right_len != 0
            && !dims_agree(
                left.dims.get(1..).unwrap_or_default(),
                right.dims.get(1..).unwrap_or_default(),
            )
            && (self.is_reactive(a)? || self.is_reactive(b)?)
        {
            self.draw_at(
                Code::RaggedArray,
                node,
                "the rows of the joined arrays have different shapes",
            )?;
            return Ok(Fact::PLACEHOLDER);
        }
        let mut dims = inner.clone();
        if let Some(first) = dims.first_mut() {
            *first = left_len.saturating_add(*right_len);
        }
        let leaf = match unify(left.leaf, right.leaf) {
            Some(leaf) => leaf,
            None => {
                self.draw_at(
                    Code::TypeMismatch,
                    node,
                    "the elements of the array hold values of different types",
                )?;
                Leaf::Error
            }
        };
        let fixed = combined(left.fixed, right.fixed, leaf == Leaf::Float);
        let read = reads(left.read, right.read, leaf == Leaf::Float);
        Ok(Fact {
            leaf,
            dims,
            fixed,
            read,
            node: false,
            family: None,
        })
    }

    fn choice_fact(&mut self, node: u32, condition: u32, then: u32, otherwise: u32) -> Stage<Fact> {
        let test = self.fact(condition)?;
        let (first, second) = (self.fact(then)?, self.fact(otherwise)?);
        if !test.dims.is_empty() {
            self.draw_at(
                Code::ConditionType,
                node,
                "the condition of an `if` is a `bool`, not an array",
            )?;
        } else if !matches!(test.leaf, Leaf::Bool | Leaf::Placeholder | Leaf::Error) {
            self.draw_at_root(Code::ConditionType, condition, "the condition is a `bool`")?;
        }
        let refused = !test.dims.is_empty() || test.leaf != Leaf::Bool;
        if first.leaf == Leaf::Placeholder || second.leaf == Leaf::Placeholder {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        let variable_arm =
            first.dims.first() == Some(&ANY_LENGTH) || second.dims.first() == Some(&ANY_LENGTH);
        if first.dims.len() != second.dims.len()
            && first.leaf != Leaf::Error
            && second.leaf != Leaf::Error
        {
            let summary = if variable_arm && unify(first.leaf, second.leaf).is_none() {
                "the branches of an `if` hold values of different types"
            } else {
                "the branches hold arrays of different shapes"
            };
            self.draw_at(Code::TypeMismatch, node, summary)?;
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        if first.leaf == Leaf::Error || second.leaf == Leaf::Error {
            return Ok(Ok(Fact::ERROR));
        }
        let Some(leaf) = unify(first.leaf, second.leaf) else {
            self.draw_at(
                Code::TypeMismatch,
                node,
                "the branches of an `if` hold values of different types",
            )?;
            return Ok(Ok(Fact::ERROR));
        };
        let fixed = combined(first.fixed, second.fixed, leaf == Leaf::Float);
        let read = reads(first.read, second.read, leaf == Leaf::Float);
        if refused {
            if first.dims != second.dims {
                return Ok(Ok(Fact::ERROR));
            }
            let kept = Fact {
                leaf,
                dims: first.dims,
                fixed,
                read,
                node: false,
                family: None,
            };
            return Ok(Ok(self.keep(node, kept)?));
        }
        if first.dims == second.dims {
            return Ok(Ok(Fact {
                leaf,
                dims: first.dims,
                fixed,
                read,
                node: false,
                family: None,
            }));
        }
        let taken = match self.evaluate(condition, EvalTy::Bool)? {
            Ok(Value::Bool(true)) => first.dims,
            Ok(Value::Bool(false)) => second.dims,
            Ok(Value::Error)
                if self.inst != NONE
                    || (self.in_body(condition).is_ok_and(|key| key.is_none())
                        && self.is_reactive(condition)?) =>
            {
                first
                    .dims
                    .iter()
                    .zip(&second.dims)
                    .map(|(a, b)| if a == b { *a } else { ANY_LENGTH })
                    .collect()
            }
            Ok(
                Value::Int(_)
                | Value::F32(_)
                | Value::F64(_)
                | Value::Enum(..)
                | Value::Str(_)
                | Value::Node(_)
                | Value::Array(_)
                | Value::Error,
            ) => return Ok(Ok(Fact::ERROR)),
            Err(declined) => return Ok(Err(declined)),
        };
        Ok(Ok(Fact {
            leaf,
            dims: taken,
            fixed,
            read,
            node: false,
            family: None,
        }))
    }

    fn kept_slice(&mut self, node: u32, base: &Fact) -> Result<Fact, Internal> {
        if base.dims.is_empty() || matches!(base.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Fact::PLACEHOLDER);
        }
        let mut kept = base.clone();
        if let Some(first) = kept.dims.first_mut() {
            *first = ANY_LENGTH;
        }
        self.keep(node, kept)
    }

    fn slice_fact(&mut self, node: u32, a: u32, start: u32, flags: u32) -> Stage<Fact> {
        let base = self.fact(a)?;
        let bounds = [self.kid(start)?, self.kid(start + 1)?];
        let mut facts = Vec::new();
        for bound in bounds.into_iter().filter(|bound| *bound != NONE) {
            facts.push((bound, self.fact(bound)?));
        }
        let faulty = |fact: &Fact| matches!(fact.leaf, Leaf::Placeholder | Leaf::Error);
        let not_an_array = base.dims.is_empty()
            && (!faulty(&base) || (base.leaf == Leaf::Placeholder && self.bool_valued(a)?));
        if not_an_array {
            self.draw_at(Code::NotAnArray, a, "only an array is sliced")?;
        }
        if facts
            .iter()
            .any(|(_, fact)| !fact.dims.is_empty() && fact.leaf != Leaf::Error)
        {
            self.draw_at(
                Code::IndexType,
                node,
                "a slice bound is an integer, not an array",
            )?;
            return Ok(Ok(self.kept_slice(node, &base)?));
        }
        if self.named_placeholder(a, &base)? {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        if not_an_array {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        let mut refused = faulty(&base);
        for (bound, fact) in &facts {
            if faulty(fact) {
                refused = true;
            } else if !fact.dims.is_empty() || matches!(fact.leaf, Leaf::Float | Leaf::Bool) {
                self.draw_at_root(Code::IndexType, *bound, "a slice bound is an integer")?;
                refused = true;
            }
        }
        let mut named = false;
        for (bound, fact) in &facts {
            if fact.leaf == Leaf::Placeholder && self.holds_name_placeholder(*bound)? {
                named = true;
            }
        }
        if refused && named {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        if refused {
            return Ok(Ok(self.kept_slice(node, &base)?));
        }
        let length = base.dims.first().copied().unwrap_or(0);
        if length == ANY_LENGTH {
            return Ok(Ok(base));
        }
        let mut window = [0, i128::from(length)];
        for (side, bound) in bounds.into_iter().enumerate() {
            if bound == NONE {
                continue;
            }
            let value = match self.evaluate(bound, EvalTy::Exact)? {
                Ok(Value::Int(value)) if i64::try_from(value).is_ok() => value,
                Ok(
                    Value::Int(_)
                    | Value::F32(_)
                    | Value::F64(_)
                    | Value::Bool(_)
                    | Value::Enum(..)
                    | Value::Str(_)
                    | Value::Node(_)
                    | Value::Array(_)
                    | Value::Error,
                ) => continue,
                Err(declined) => return Ok(Err(declined)),
            };
            let from_end = flags & (1 << side) != 0;
            let position = if from_end {
                i128::from(length) - value
            } else {
                value
            };
            if let Some(slot) = window.get_mut(side) {
                *slot = position.clamp(0, i128::from(length));
            }
        }
        let [low, high] = window;
        let sliced = u32::try_from(high.max(low) - low).unwrap_or(0);
        let mut dims = base.dims.clone();
        if let Some(first) = dims.first_mut() {
            *first = sliced;
        }
        Ok(Ok(Fact {
            leaf: base.leaf,
            dims,
            fixed: base.fixed,
            read: base.read,
            node: false,
            family: None,
        }))
    }

    fn bool_valued(&self, node: u32) -> Result<bool, Internal> {
        let (op, _, _, _) = self.ir.node(node)?;
        Ok(matches!(
            op,
            Op::Lt | Op::Le | Op::Gt | Op::Ge | Op::Is | Op::IsNot | Op::And | Op::Or | Op::Not
        ) && !self.holds_a_missing_operand(node)?)
    }

    fn named_placeholder(&self, node: u32, fact: &Fact) -> Result<bool, Internal> {
        let (op, _, _, _) = self.ir.node(node)?;
        Ok(fact.leaf == Leaf::Placeholder && op == Op::Name)
    }

    fn index_fact(&mut self, node: u32, a: u32, b: u32) -> Result<Fact, Internal> {
        let base = self.fact(a)?;
        let index = self.fact(b)?;
        if !index.dims.is_empty() && !matches!(index.leaf, Leaf::Placeholder | Leaf::Error) {
            self.draw_at(
                Code::IndexType,
                node,
                "an index is an integer, not an array",
            )?;
            let known_base = !matches!(base.leaf, Leaf::Placeholder | Leaf::Error)
                || (base.leaf == Leaf::Placeholder && self.bool_valued(a)?);
            if base.dims.is_empty() && known_base && !self.named_placeholder(a, &base)? {
                self.draw_at(Code::NotAnArray, a, "only an array is indexed")?;
            }
            if !base.dims.is_empty() && !matches!(base.leaf, Leaf::Placeholder | Leaf::Error) {
                return self.keep_kind(node, element_of(&base));
            }
            return Ok(Fact::PLACEHOLDER);
        }
        if self.named_placeholder(a, &base)? {
            return Ok(Fact::PLACEHOLDER);
        }
        let known_base = !matches!(base.leaf, Leaf::Placeholder | Leaf::Error)
            || (base.leaf == Leaf::Placeholder && self.bool_valued(a)?);
        if base.dims.is_empty() && known_base {
            self.draw_at(Code::NotAnArray, a, "only an array is indexed")?;
            return Ok(Fact::PLACEHOLDER);
        }
        let faulty_index = matches!(index.leaf, Leaf::Placeholder | Leaf::Error);
        let refused = !faulty_index
            && (!index.dims.is_empty() || matches!(index.leaf, Leaf::Float | Leaf::Bool));
        if refused {
            self.draw_at_root(Code::IndexType, b, "an index is an integer")?;
        }
        if base.leaf == Leaf::Placeholder || base.dims.is_empty() {
            return Ok(Fact::PLACEHOLDER);
        }
        let element = Fact {
            leaf: base.leaf,
            dims: base.dims.get(1..).unwrap_or_default().to_vec(),
            fixed: base.fixed,
            read: base.read,
            node: base.node,
            family: None,
        };
        if index.leaf == Leaf::Placeholder {
            if base.leaf == Leaf::Error {
                return Ok(Fact::PLACEHOLDER);
            }
            return self.keep_kind(node, element);
        }
        if !refused
            && !faulty_index
            && self.is_reactive(a)?
            && !self.is_reactive(b)?
            && let Some((array, slices)) = self.view(a)?
            && slices.is_empty()
            && let Ok(Value::Int(position)) = self.evaluate(b, EvalTy::Exact)?
        {
            let (_, start, count, _) = self.ir.node(array)?;
            let (read_op, _, _, _) = self.ir.node(node)?;
            let position = if read_op == Op::IndexFromEnd {
                i128::from(count) - position
            } else {
                position
            };
            if let Ok(position) = u32::try_from(position)
                && position < count
                && self.is_reactive(self.kid(start + position)?)?
            {
                return self.fact(self.kid(start + position)?);
            }
        }
        if refused || faulty_index {
            return self.keep(node, element);
        }
        Ok(element)
    }

    fn array_fact(&mut self, node: u32, start: u32, count: u32) -> Result<Fact, Internal> {
        if count == 0 {
            return Ok(Fact {
                leaf: Leaf::Empty,
                dims: vec![0],
                fixed: None,
                read: None,
                node: false,
                family: None,
            });
        }
        let mut facts = Vec::new();
        for at in start..start + count {
            facts.push(self.fact(self.kid(at)?)?);
        }
        if facts.iter().any(|fact| {
            fact.leaf == Leaf::Placeholder || (fact.leaf == Leaf::Error && !fact.dims.is_empty())
        }) {
            return Ok(Fact::PLACEHOLDER);
        }
        let Some(first) = facts.first().cloned() else {
            return Err(Internal::missed("array element"));
        };
        let mut shapes = Vec::with_capacity(facts.len());
        for (at, fact) in (start..start + count).zip(&facts) {
            let kid = self.kid(at)?;
            let variable = !fact.dims.is_empty() && self.variable_length(kid)?;
            let mut dims = fact.dims.clone();
            if let Some(length) = dims.first_mut().filter(|_| variable) {
                *length = ANY_LENGTH;
            }
            shapes.push((kid, variable, dims));
        }
        let fixed_first = !first.dims.is_empty() && shapes.first().is_some_and(|shape| !shape.1);
        let mut ragged = false;
        if shapes.iter().any(|(_, variable, dims)| {
            (!fixed_first || !*variable) && !dims_agree(dims, &first.dims)
        }) {
            self.draw_at(Code::RaggedArray, node, RAGGED)?;
            ragged = true;
        }
        for (kid, variable, _) in shapes.iter().skip(1) {
            if fixed_first && *variable {
                self.draw_at(Code::RaggedArray, *kid, RAGGED)?;
                ragged = true;
            }
        }
        if ragged {
            return Ok(Fact::PLACEHOLDER);
        }
        let mut dims = vec![count];
        dims.extend(first.dims.iter().copied());
        let mut leaf = first.leaf;
        for fact in facts.iter().skip(1) {
            let Some(unified) = unify(leaf, fact.leaf) else {
                self.draw_at(
                    Code::TypeMismatch,
                    node,
                    "the elements of the array hold values of different types",
                )?;
                return Ok(Fact {
                    leaf: Leaf::Error,
                    dims,
                    fixed: None,
                    read: None,
                    node: false,
                    family: None,
                });
            };
            leaf = unified;
        }
        let (mut fixed, mut read) = (None, None);
        for fact in &facts {
            fixed = combined(fixed, fact.fixed, leaf == Leaf::Float);
            read = reads(read, fact.read, leaf == Leaf::Float);
        }
        Ok(Fact {
            leaf,
            dims,
            fixed,
            read,
            node: false,
            family: None,
        })
    }
}

fn leaf_of(leaf: Leaf) -> Leaf {
    if leaf == Leaf::Empty { Leaf::Int } else { leaf }
}

fn numeric_leaf(a: Leaf, b: Leaf) -> Leaf {
    if leaf_of(a) == Leaf::Int && leaf_of(b) == Leaf::Int {
        Leaf::Int
    } else {
        Leaf::Float
    }
}

pub(crate) fn unify(a: Leaf, b: Leaf) -> Option<Leaf> {
    match (a, b) {
        (Leaf::Placeholder, _) | (_, Leaf::Placeholder) => Some(Leaf::Placeholder),
        (Leaf::Error, _) | (_, Leaf::Error) => Some(Leaf::Error),
        (Leaf::Empty, other) | (other, Leaf::Empty) => Some(other),
        (Leaf::Bool, Leaf::Bool) => Some(Leaf::Bool),
        (Leaf::Bool, _) | (_, Leaf::Bool) => None,
        (Leaf::Enum(Some(x)), Leaf::Enum(Some(y))) => (x == y).then_some(a),
        (Leaf::Enum(Some(_)), Leaf::Enum(None)) => Some(a),
        (Leaf::Enum(None), Leaf::Enum(_)) => Some(b),
        (Leaf::Enum(_), _) | (_, Leaf::Enum(_)) => None,
        (x, y) => Some(numeric_leaf(x, y)),
    }
}

pub(super) fn operand_message(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "`+` takes two numbers or two strings",
        BinaryOp::Div => "`/` takes numbers",
        BinaryOp::IntDiv => "`div` takes integers",
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
            "`<`, `<=`, `>` and `>=` compare numbers"
        }
        BinaryOp::Is | BinaryOp::IsNot => {
            "`is` compares two numbers, `bool`s, values of one enum or strings"
        }
        BinaryOp::And | BinaryOp::Or => "`and` and `or` take `bool`s",
        BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Rem | BinaryOp::Pow => {
            "this operator takes numbers"
        }
    }
}

impl Core<'_> {
    pub fn holds_a_missing_operand(&self, node: u32) -> Result<bool, Internal> {
        let (op, _, _, _) = self.ir.node(node)?;
        if matches!(op, Op::Call | Op::NamedCall | Op::For) {
            return Ok(false);
        }
        for operand in self.operands(node)? {
            if self.ir.node(operand)?.0 == Op::Bad {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Core<'_> {
    fn keep_kind(&mut self, node: u32, fact: Fact) -> Result<Fact, Internal> {
        if !self.holds_a_missing_operand(node)? {
            let key = (self.inst_key(node)?, node);
            self.placeholder_kinds.insert(key, fact);
        }
        Ok(Fact::PLACEHOLDER)
    }

    pub fn placeholder_kind(&self, node: u32) -> Result<Option<Fact>, Internal> {
        let key = (self.inst_key(node)?, node);
        Ok(self.placeholder_kinds.get(&key).cloned())
    }
}

fn element_of(base: &Fact) -> Fact {
    Fact {
        leaf: base.leaf,
        dims: base.dims.get(1..).unwrap_or_default().to_vec(),
        fixed: base.fixed,
        read: base.read,
        node: base.node,
        family: None,
    }
}

impl Core<'_> {
    fn reactive_local_reference(&mut self, node: u32) -> Result<bool, Internal> {
        let Some(rec) = self.ir.lets.iter().find(|rec| rec.value == node).copied() else {
            return Ok(false);
        };
        let span = self.span(self.file_of(node), rec.lo, rec.hi);
        self.draw(
            Code::ReactiveNodeReference,
            span,
            "a value holding node references reads no cell; write the choice in the destination",
        );
        Ok(true)
    }
}

impl Core<'_> {
    fn cell_size_fact(&mut self, node: u32, at: u32) -> Stage<Fact> {
        let (_, _, field, _) = self.ir.node(node)?;
        let text = self.ir.name_text(field).unwrap_or_default().to_string();
        if let Some(uses) = self.cell_uses.get_mut(at as usize) {
            uses[0] = true;
        }
        if self.derived(at) && !self.has_fact(self.cell_root(at)?)? {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        if self.cell_annotation_ty(at)?.is_none() && !self.has_fact(self.cell_root(at)?)? {
            return Ok(Err(Declined(Reason::Member)));
        }
        let fact = self.cell_read_fact(at)?;
        if matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            return Ok(Ok(Fact::PLACEHOLDER));
        }
        match fact.dims.first().copied() {
            None => {
                let declared = self
                    .ir
                    .name(field)
                    .and_then(|entry| self.decl.get(entry.sym.0 as usize))
                    .is_some_and(|decl| *decl != DeclRef::None);
                if declared {
                    return Ok(Err(Declined(Reason::Member)));
                }
                self.draw_at(
                    Code::UnknownAttribute,
                    node,
                    format!(
                        "`.{text}` reads an attribute of an instance or a wire, and this is a value"
                    ),
                )?;
                Ok(Ok(Fact::PLACEHOLDER))
            }
            Some(ANY_LENGTH) if text == "size" => {
                self.runtime_members.insert(node);
                Ok(Ok(Fact::fixed(Leaf::Int, Some(EvalTy::I32))))
            }
            Some(ANY_LENGTH) => Ok(Err(Declined(Reason::Member))),
            Some(length) => {
                self.array_members.insert(node, i128::from(length));
                Ok(Ok(Fact::scalar(Leaf::Int)))
            }
        }
    }
}

impl Core<'_> {
    fn dot_call_lands(&self, node: u32) -> Result<bool, Internal> {
        let (_, callee, _, _) = self.ir.node(node)?;
        let (_, name, _, _) = self.ir.node(callee)?;
        let text = self.ir.name_text(name).unwrap_or_default();
        if super::calls::MEMBER_NAMES.contains(&text) {
            return Ok(false);
        }
        let Some(receiver) = self.arguments(node)?.first().copied() else {
            return Ok(false);
        };
        let callable = self.callee(node)?.is_some()
            || self.math_call(node)?
            || self.math_miscount(node)?.is_some()
            || self.math_named(node)?.is_some();
        let receiver_fact = self.fact(receiver)?;
        let faulty = self.ir.node(receiver)?.0 == Op::To
            || (self.math_named(node)?.is_none()
                && matches!(receiver_fact.leaf, Leaf::Placeholder | Leaf::Error));
        Ok(!receiver_fact.node && callable && !(faulty && self.callee(node)?.is_none()))
    }
}

impl Core<'_> {
    fn member_dot_call(&mut self, node: u32) -> Result<bool, Internal> {
        let (op, callee, _, _) = self.ir.node(node)?;
        let (_, name, _, _) = self.ir.node(callee)?;
        let text = self.ir.name_text(name).unwrap_or_default().to_string();
        let arguments = self.arguments(node)?;
        let (Op::Call, [receiver, ..]) = (op, arguments.as_slice()) else {
            return Ok(false);
        };
        let method = self.unknown_methods.contains(&node);
        if !method
            && !matches!(
                text.as_str(),
                "size" | "capacity" | "bytes" | "keys" | "values" | "map"
            )
            || self.ir.node(*receiver)?.0 == Op::To
        {
            return Ok(false);
        }
        let fact = self.fact(*receiver)?;
        let typed = match fact.leaf {
            Leaf::Int | Leaf::Float | Leaf::Bool | Leaf::Empty | Leaf::Enum(Some(_)) => true,
            Leaf::Placeholder | Leaf::Error | Leaf::Enum(None) => false,
        };
        if method && self.reads_a_cyclic_cell(*receiver)? {
            let file = self.file_of(node);
            let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
            let span = self.span(file, entry.lo, entry.hi);
            self.draw(
                Code::UnknownName,
                span,
                format!("`{text}` names nothing in scope"),
            );
            return Ok(true);
        }
        if method && !fact.node && matches!(fact.leaf, Leaf::Placeholder | Leaf::Error) {
            let let_read = self.ir.node(*receiver)?.0 == Op::Name
                && matches!(self.bound(*receiver)?, DeclRef::Let(_));
            return Ok(!let_read);
        }
        if fact.node || !typed {
            return Ok(false);
        }
        let array = !fact.dims.is_empty() || fact.leaf == Leaf::Empty;
        if array && text == "map" {
            let span = self.node_span(node)?;
            self.draw(
                Code::UnbuiltConstruct,
                span,
                "an array's `map` (SPEC §5.7) is a construct this compiler does not build",
            );
        } else if array && matches!(text.as_str(), "size" | "capacity" | "bytes") {
            let file = self.file_of(node);
            let (lo, _) = self.ir.span(node)?;
            let (_, hi) = self.ir.span(callee)?;
            let span = self.span(file, lo, hi);
            self.draw(
                Code::NotCallable,
                span,
                "only a function or a built-in is called",
            );
        } else {
            let file = self.file_of(node);
            let entry = self.ir.name(name).ok_or(Internal::missed("IR name"))?;
            let span = self.span(file, entry.lo, entry.hi);
            self.draw(
                Code::UnknownName,
                span,
                format!("`{text}` names nothing in scope"),
            );
        }
        Ok(true)
    }
}

impl Core<'_> {
    fn reads_a_cyclic_cell(&self, receiver: u32) -> Result<bool, Internal> {
        if self.ir.node(receiver)?.0 != Op::Name {
            return Ok(false);
        }
        let DeclRef::Cell(at) = self.bound(receiver)? else {
            return Ok(false);
        };
        Ok(self.is_cyclic_cell(at))
    }
}
