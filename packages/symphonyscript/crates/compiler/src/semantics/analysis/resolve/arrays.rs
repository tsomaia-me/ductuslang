use symphonyscript_core::{BinaryOp, Builtin, Scalar};

use super::annotation::Annotation;
use super::layout_found::LayoutFound;
use super::length_limit::LENGTH_LIMIT;
use super::native_layout::native_layout;
use super::shaped::Shaped;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::assumed::Assumed;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::lengths_of::lengths_of;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::settling::Settling;
use crate::semantics::analysis::shape_fault::ShapeFault;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, Loc, Number, SliceBound, StateId, Suffix};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn join_val(
        &mut self,
        expr: ExprId,
        (lhs, rhs): (Val, Val),
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        if let (Val::Array(left), Val::Array(right)) = (&lhs, &rhs) {
            let inner = if left.elements.is_empty() && left.shape.inner().is_scalar() {
                right.shape.inner()
            } else {
                left.shape.inner()
            };
            let right_inner = if right.elements.is_empty() && right.shape.inner().is_scalar() {
                inner.clone()
            } else {
                right.shape.inner()
            };
            if inner != right_inner {
                self.error(Code::RaggedArray, loc, scope, JOIN_ROWS)?;
                return Ok(Val::Error);
            }
            let length = left
                .shape
                .outer()
                .unwrap_or(0)
                .saturating_add(right.shape.outer().unwrap_or(0));
            if length >= LENGTH_LIMIT {
                self.error(Code::LengthLimit, loc, scope, LENGTH)?;
                return Ok(Val::Error);
            }
            let mut elements = left.elements.clone();
            elements.extend(right.elements.iter().copied());
            let group = self.push_node(Op::Array(elements.clone()), loc, scope)?;
            self.memo.literal_groups.insert((expr, scope), group);
            self.memo.group_list.push(group);
            return Ok(Val::array(inner.wrap(length), elements));
        }
        let (Some(left), Some(right)) =
            (self.boxed(&lhs, loc, scope)?, self.boxed(&rhs, loc, scope)?)
        else {
            return Ok(Val::Error);
        };
        self.scalar(Op::Join(left, right), loc, scope)
    }

    pub(in crate::semantics::analysis) fn is_array_val(&self, val: &Val) -> Result<bool, Internal> {
        Ok(match val {
            Val::Array(_) => true,
            Val::Scalar(node) => self.definite_array(*node)?,
            Val::Error => false,
        })
    }

    pub(in crate::semantics::analysis) fn definite_array(
        &self,
        root: NodeId,
    ) -> Result<bool, Internal> {
        let mut node = root;
        loop {
            if !self.arrays.contains(&node) {
                return Ok(false);
            }
            match &self.node(node)?.op {
                Op::Use(inner) | Op::Chosen(inner) | Op::Annotate(_, inner) => node = *inner,
                Op::Read(binding) => {
                    if self.memo.binding_layouts.contains_key(binding) {
                        return Ok(true);
                    }
                    node = self.binding(*binding)?.init;
                }
                Op::Join(..)
                | Op::Slice { .. }
                | Op::Array(_)
                | Op::History { .. }
                | Op::Native { .. }
                | Op::Builtin(Builtin::Bytes, _) => return Ok(true),
                Op::If(_, then, otherwise) => {
                    return Ok(self.definite_array(*then)? && self.definite_array(*otherwise)?);
                }
                Op::Match {
                    arms, otherwise, ..
                } => {
                    for (_, value) in arms {
                        if !self.definite_array(*value)? {
                            return Ok(false);
                        }
                    }
                    return Ok(match otherwise {
                        Some(value) => self.definite_array(*value)?,
                        None => true,
                    });
                }
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Bare(_)
                | Op::Pi
                | Op::NodeRef(_)
                | Op::Arg(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::Pick { .. }
                | Op::Element { .. }
                | Op::Convert(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::ToRead { .. } => return Ok(false),
            }
        }
    }

    pub(in crate::semantics::analysis) fn length_val(
        &mut self,
        val: Val,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let node = match &val {
            Val::Array(array) => {
                let length = array.shape.outer().unwrap_or(0);
                return Ok(Some(self.scalar(
                    Op::Number(Number::Int(i128::from(length)), Suffix::None),
                    loc,
                    scope,
                )?));
            }
            Val::Error => return Ok(Some(Val::Error)),
            Val::Scalar(node) => *node,
        };
        if let Some(known) = self.memo.lengths.get(&node) {
            return Ok(Some(known.clone()));
        }
        if !self.arrays.contains(&node) {
            self.error(Code::NotAnArray, loc, scope, NOT_AN_ARRAY)?;
            return Ok(Some(Val::Error));
        }
        let found = some!(self.layout_of(node, need)?);
        let certain = found.certain;
        let length = self.length_of_layout(node, found, (loc, scope))?;
        if certain {
            self.memo.lengths.insert(node, length.clone());
        }
        Ok(Some(length))
    }

    fn length_of_layout(
        &mut self,
        node: NodeId,
        found: LayoutFound,
        (loc, scope): (Loc, ScopeId),
    ) -> Result<Val, Internal> {
        match found.shaped {
            Ok(Some(layout)) if !layout.variable && found.certain => {
                let length = layout.outer().unwrap_or(0);
                self.scalar(
                    Op::Number(Number::Int(i128::from(length)), Suffix::None),
                    loc,
                    scope,
                )
            }
            Ok(Some(_)) => self.scalar(Op::Builtin(Builtin::Len, vec![node]), loc, scope),
            Ok(None) => {
                self.error(Code::NotAnArray, loc, scope, NOT_AN_ARRAY)?;
                Ok(Val::Error)
            }
            Err(fault) => {
                if let Some(fault) = fault {
                    self.error_at_node(fault.code, fault.node, fault.message)?;
                }
                Ok(Val::Error)
            }
        }
    }

    pub(in crate::semantics::analysis) fn bytes_val(
        &mut self,
        val: Val,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        let Some(node) = self.boxed(&val, loc, scope)? else {
            return Ok(Val::Error);
        };
        self.scalar(Op::Builtin(Builtin::Bytes, vec![node]), loc, scope)
    }

    pub(in crate::semantics::analysis) fn as_bytes(
        &mut self,
        val: Val,
        expected: Option<&Annotation>,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let Some(expected) = expected else {
            return Ok(Some(val));
        };
        let wants_bytes = expected.scalar == Scalar::U8
            && expected.lengths.len() <= 1
            && (expected.lengths.len() + usize::from(expected.capacity.is_some())) == 1;
        let Val::Scalar(node) = val else {
            return Ok(Some(val));
        };
        if !wants_bytes || self.arrays.contains(&node) {
            return Ok(Some(val));
        }
        if let Some(bytes) = self.memo.string_bytes.get(&node) {
            return Ok(Some(Val::Scalar(*bytes)));
        }
        let sig = some!(self.sig(node, need));
        if sig.kind != Kind::Str || sig.reactive {
            return Ok(Some(val));
        }
        let bytes = self.scalar(Op::Builtin(Builtin::Bytes, vec![node]), loc, scope)?;
        if let Val::Scalar(created) = bytes {
            self.memo.string_bytes.insert(node, created);
        }
        Ok(Some(bytes))
    }

    pub(in crate::semantics::analysis) fn counted_from_end(
        &mut self,
        base: &Val,
        offset: NodeId,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<NodeId>, Internal> {
        let length = match self.length_val(base.clone(), loc, scope, need)? {
            Some(Val::Scalar(length)) => length,
            Some(Val::Array(_) | Val::Error) => return Ok(Some(offset)),
            None => return Ok(None),
        };
        Ok(Some(self.push_node(
            Op::Binary(BinaryOp::Sub, length, offset),
            loc,
            scope,
        )?))
    }

    pub(in crate::semantics::analysis) fn slice(
        &mut self,
        base: ExprId,
        bounds: (Option<SliceBound>, Option<SliceBound>),
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let base_val = self.val(Query::Expr(base, scope), need);
        let lower = bounds
            .0
            .map(|bound| self.val(Query::Expr(bound.value, scope), need));
        let upper = bounds
            .1
            .map(|bound| self.val(Query::Expr(bound.value, scope), need));
        let base_val = some!(base_val);
        let lower = match lower {
            Some(found) => Some(some!(found)),
            None => None,
        };
        let upper = match upper {
            Some(found) => Some(some!(found)),
            None => None,
        };
        let mut nodes = Vec::new();
        for (bound, val) in [(bounds.0, lower), (bounds.1, upper)] {
            let (Some(bound), Some(val)) = (bound, val) else {
                nodes.push(None);
                continue;
            };
            self.single_operand(&val, Code::IndexType, BOUND_TYPE);
            match val {
                Val::Scalar(node) => nodes.push(Some((node, bound.from_end))),
                Val::Array(_) => {
                    self.error(Code::IndexType, loc, scope, BOUND_TYPE)?;
                    return Ok(Some(Val::Error));
                }
                Val::Error => return Ok(Some(Val::Error)),
            }
        }
        let (lower, upper) = (
            nodes.first().copied().flatten(),
            nodes.get(1).copied().flatten(),
        );
        match &base_val {
            Val::Error => return Ok(Some(Val::Error)),
            Val::Scalar(node) if !self.arrays.contains(node) => {
                self.error(
                    Code::NotAnArray,
                    self.hir.expr(base)?.loc,
                    scope,
                    "only an array is sliced",
                )?;
                return Ok(Some(Val::Error));
            }
            Val::Scalar(_) | Val::Array(_) => {}
        }
        if let Val::Array(array) = &base_val {
            let length = i128::from(array.shape.outer().unwrap_or(0));
            let mut known = Vec::new();
            for bound in [lower, upper] {
                known.push(match bound {
                    None => Some(None),
                    Some((node, from_end)) => some!(self.known_bound(node, need)?)
                        .map(|value| Some(if from_end { length - value } else { value })),
                });
            }
            if let [Some(start), Some(end)] = known.as_slice() {
                let start = start.unwrap_or(0).clamp(0, length);
                let end = end.unwrap_or(length).clamp(0, length);
                let end = end.max(start);
                let (start, end) = (
                    usize::try_from(start).map_err(|_| Internal::missed("slice bound"))?,
                    usize::try_from(end).map_err(|_| Internal::missed("slice bound"))?,
                );
                let rows: Vec<Val> = (start..end)
                    .map(|row| base_val.slice(row))
                    .collect::<Result<_, _>>()?;
                let group = self.value_group(base, &base_val, scope)?;
                let mut reads = Vec::new();
                for row in &rows {
                    for leaf in row.nodes() {
                        reads.push(self.chosen_read(leaf, group, loc, scope)?);
                    }
                }
                let count = u32::try_from(end - start).map_err(|_| Internal::missed("slice"))?;
                return Ok(Some(Val::array(array.shape.inner().wrap(count), reads)));
            }
        }
        let Some(whole) = self.boxed(&base_val, loc, scope)? else {
            return Ok(Some(Val::Error));
        };
        Ok(Some(self.scalar(
            Op::Slice {
                base: whole,
                lower,
                upper,
            },
            loc,
            scope,
        )?))
    }

    fn known_bound(
        &mut self,
        node: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<i128>>, Internal> {
        let sig = some!(self.sig(node, need));
        if sig.is_error() || sig.reactive || sig.frame || sig.kind != Kind::Int {
            return Ok(Some(None));
        }
        Ok(Some(
            match some!(self.cval(Query::Eval(node, EvalTy::Exact), need)) {
                CVal::Int(value) if i64::try_from(value).is_ok() => Some(value),
                CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Int(_)
                | CVal::Array(_)
                | CVal::Error => None,
            },
        ))
    }

    pub(in crate::semantics::analysis) fn layout_of(
        &mut self,
        root: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<LayoutFound>, Internal> {
        self.layout_in(root, None, need)
    }

    fn cached_layout(&self, node: NodeId, assumed: Option<&Assumed>) -> Option<LayoutFound> {
        self.memo
            .layouts
            .get(&node)
            .filter(|(known, epoch)| {
                known.certain || (assumed.is_none() && *epoch == self.memo.settle_epoch)
            })
            .map(|(known, _)| known.clone())
    }

    fn unsettled_state(&self, op: &Op) -> Result<Option<StateId>, Internal> {
        Ok(match op {
            Op::Read(binding) => match self.binding(*binding)?.kind {
                BindingKind::State { state } if !self.memo.settled.contains(binding) => Some(state),
                BindingKind::State { .. }
                | BindingKind::Input { .. }
                | BindingKind::Let
                | BindingKind::Param
                | BindingKind::Attr { .. } => None,
            },
            Op::History {
                cell: HistoryCell::State(state),
                ..
            } => match self.state_binding(*state) {
                Some(binding) if self.memo.settled.contains(&binding) => None,
                Some(_) | None => Some(*state),
            },
            _ => None,
        })
    }

    pub(in crate::semantics::analysis) fn layout_in(
        &mut self,
        root: NodeId,
        assumed: Option<&Assumed>,
        need: &mut Vec<Query>,
    ) -> Result<Option<LayoutFound>, Internal> {
        if let Some(known) = self.cached_layout(root, assumed) {
            return Ok(Some(known));
        }
        let mut found: FastMap<NodeId, LayoutFound> = FastMap::default();
        let mut stack = vec![(root, false)];
        let mut missing = false;
        while let Some((node, ready)) = stack.pop() {
            if found.contains_key(&node) {
                continue;
            }
            if let Some(known) = self.cached_layout(node, assumed) {
                found.insert(node, known);
                continue;
            }
            if !self.arrays.contains(&node) {
                found.insert(node, scalar_found());
                continue;
            }
            let Some(sig) = self.sig(node, need) else {
                missing = true;
                continue;
            };
            if sig.is_error() {
                found.insert(node, fault(None));
                continue;
            }
            if !sig.reactive && !sig.frame {
                let Some(value) = self.cval(Query::Eval(node, EvalTy::Default), need) else {
                    missing = true;
                    continue;
                };
                let shaped = match &value {
                    CVal::Error => Err(None),
                    CVal::Array(_) => Ok(Some(Layout::fixed(lengths_of(&value)))),
                    CVal::Int(_)
                    | CVal::F32(_)
                    | CVal::F64(_)
                    | CVal::Bool(_)
                    | CVal::Enum(..)
                    | CVal::Str(_)
                    | CVal::Node(_) => Ok(None),
                };
                found.insert(
                    node,
                    LayoutFound {
                        shaped,
                        certain: true,
                    },
                );
                continue;
            }
            let op = self.node(node)?.op.clone();
            if !ready
                && assumed.is_none()
                && let Some(state) = self.unsettled_state(&op)?
                && self.settle_from(state, need)? == Settling::Pending
            {
                missing = true;
                continue;
            }
            let parts = self.layout_parts(&op, assumed)?;
            if !ready {
                stack.push((node, true));
                stack.extend(parts.iter().map(|part| (*part, false)));
                continue;
            }
            let mut inputs = Vec::with_capacity(parts.len());
            for part in &parts {
                match found.get(part) {
                    Some(part) => inputs.push(part.clone()),
                    None => missing = true,
                }
            }
            if missing {
                continue;
            }
            let result = some!(self.combined_layout(node, &op, (&inputs, assumed), need)?);
            found.insert(node, result);
        }
        if missing {
            return Ok(None);
        }
        let result = found
            .get(&root)
            .cloned()
            .ok_or(Internal::missed("layout"))?;
        if assumed.is_none() {
            let epoch = self.memo.settle_epoch;
            self.memo.layouts.extend(
                found
                    .into_iter()
                    .map(|(node, known)| (node, (known, epoch))),
            );
        }
        Ok(Some(result))
    }

    fn layout_parts(&self, op: &Op, assumed: Option<&Assumed>) -> Result<Vec<NodeId>, Internal> {
        Ok(match op {
            Op::Read(binding) => {
                let member = assumed.and_then(|assumed| assumed.members.get(binding));
                if member == Some(&true)
                    || (member.is_none() && self.memo.binding_layouts.contains_key(binding))
                {
                    Vec::new()
                } else {
                    vec![self.binding(*binding)?.init]
                }
            }
            Op::Use(operand) | Op::Chosen(operand) | Op::Annotate(_, operand) => vec![*operand],
            Op::Element { base, .. } | Op::Slice { base, .. } => vec![*base],
            Op::If(_, then, otherwise) => vec![*then, *otherwise],
            Op::Match {
                arms, otherwise, ..
            } => {
                let mut parts: Vec<NodeId> = arms.iter().map(|(_, value)| *value).collect();
                parts.extend(otherwise.iter().copied());
                parts
            }
            Op::Array(elements) => elements.clone(),
            Op::Join(lhs, rhs) => vec![*lhs, *rhs],
            Op::History { fallback, .. } => fallback.iter().copied().collect(),
            Op::Builtin(Builtin::Bytes, args) => args.clone(),
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Arg(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Pick { .. }
            | Op::Convert(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::ToRead { .. }
            | Op::Native { .. } => Vec::new(),
        })
    }

    fn combined_layout(
        &self,
        node: NodeId,
        op: &Op,
        (parts, assumed): (&[LayoutFound], Option<&Assumed>),
        need: &mut Vec<Query>,
    ) -> Result<Option<LayoutFound>, Internal> {
        let certain = parts.iter().all(|part| part.certain);
        let mut layouts = Vec::with_capacity(parts.len());
        for part in parts {
            match &part.shaped {
                Ok(layout) => layouts.push(layout.clone()),
                Err(fault) => {
                    return Ok(Some(LayoutFound {
                        shaped: Err(*fault),
                        certain,
                    }));
                }
            }
        }
        let shaped = match op {
            Op::Read(binding) => match assumed.and_then(|assumed| assumed.members.get(binding)) {
                Some(true) => Ok(self.variable_layout(*binding)),
                Some(false) => Ok(layouts.first().cloned().flatten()),
                None => match self.memo.binding_layouts.get(binding) {
                    Some(layout) => Ok(Some(layout.clone())),
                    None => Ok(layouts.first().cloned().flatten()),
                },
            },
            Op::Use(_) | Op::Chosen(_) | Op::Annotate(..) => Ok(layouts.first().cloned().flatten()),
            Op::Element { base, .. } => match layouts.first().cloned().flatten() {
                Some(layout) => {
                    let inner = layout.inner().to_vec();
                    Ok((!inner.is_empty()).then(|| Layout::fixed(inner)))
                }
                None => Err(Some(ShapeFault {
                    node: *base,
                    code: Code::NotAnArray,
                    message: "only an array is indexed",
                })),
            },
            Op::If(condition, ..)
            | Op::Match {
                scrutinee: condition,
                ..
            } => match some!(self.chosen_arm(op, need)?) {
                Some(arm) => Ok(layouts.get(arm).cloned().flatten()),
                None => {
                    let reactive = some!(self.sig(*condition, need)).reactive;
                    self.choice_layout(node, &layouts, reactive)
                }
            },
            Op::Array(elements) => {
                let first = layouts.first().cloned().flatten();
                if layouts.iter().any(|layout| *layout != first) {
                    Err(Some(ShapeFault {
                        node,
                        code: Code::RaggedArray,
                        message: RAGGED,
                    }))
                } else {
                    match first {
                        Some(layout) if layout.variable => Err(Some(ShapeFault {
                            node,
                            code: Code::VariableArray,
                            message: "the elements of an array are fixed arrays or single values",
                        })),
                        first => {
                            let mut lengths =
                                first.map(|layout| layout.lengths).unwrap_or_default();
                            lengths.push(u32::try_from(elements.len()).unwrap_or(u32::MAX));
                            Ok(Some(Layout::fixed(lengths)))
                        }
                    }
                }
            }
            Op::Join(..) => match (
                layouts.first().cloned().flatten(),
                layouts.get(1).cloned().flatten(),
            ) {
                (Some(left), Some(right)) => {
                    let (left_inner, right_inner) = (left.inner().to_vec(), right.inner().to_vec());
                    let empty = |layout: &Layout| layout.outer() == Some(0);
                    let inner = if empty(&left) && left_inner.is_empty() {
                        right_inner.clone()
                    } else {
                        left_inner.clone()
                    };
                    let agrees = right_inner == inner || (empty(&right) && right_inner.is_empty());
                    if !agrees {
                        Err(Some(ShapeFault {
                            node,
                            code: Code::RaggedArray,
                            message: JOIN_ROWS,
                        }))
                    } else {
                        match (left.outer(), right.outer()) {
                            (Some(a), Some(b)) => match a.checked_add(b) {
                                Some(length) if length < LENGTH_LIMIT => {
                                    let mut lengths = inner;
                                    lengths.push(length);
                                    Ok(Some(Layout::fixed(lengths)))
                                }
                                Some(_) | None => Err(Some(ShapeFault {
                                    node,
                                    code: Code::LengthLimit,
                                    message: LENGTH,
                                })),
                            },
                            _ => Ok(Some(Layout::variable(inner))),
                        }
                    }
                }
                _ => Err(Some(ShapeFault {
                    node,
                    code: Code::ArrayOperand,
                    message: "`+` joins two arrays, and one of these is a single value",
                })),
            },
            Op::Slice { base, lower, upper } => match layouts.first().cloned().flatten() {
                Some(layout) => {
                    let inner = layout.inner().to_vec();
                    match layout.outer() {
                        Some(length) => {
                            let start = some!(self.slice_bound_value(*lower, length, need)?);
                            let end = some!(self.slice_bound_value(*upper, length, need)?);
                            match (start, end) {
                                (Some(start), Some(end)) => {
                                    let length = i128::from(length);
                                    let start = start.unwrap_or(0).clamp(0, length);
                                    let end = end.unwrap_or(length).clamp(0, length).max(start);
                                    let mut lengths = inner;
                                    lengths.push(u32::try_from(end - start).unwrap_or(u32::MAX));
                                    Ok(Some(Layout::fixed(lengths)))
                                }
                                _ => Ok(Some(Layout::variable(inner))),
                            }
                        }
                        None => Ok(Some(Layout::variable(inner))),
                    }
                }
                None => Err(Some(ShapeFault {
                    node: *base,
                    code: Code::NotAnArray,
                    message: "only an array is sliced",
                })),
            },
            Op::History {
                cell: HistoryCell::State(state),
                fallback: Some(_),
                ..
            } => {
                let fallback = layouts.first().cloned().flatten();
                match self.state_binding(*state) {
                    Some(binding) => {
                        match assumed.and_then(|assumed| assumed.members.get(&binding)) {
                            Some(true) => Ok(
                                fallback.map(|layout| Layout::variable(layout.inner().to_vec()))
                            ),
                            Some(false) => Ok(fallback),
                            None if self.memo.settled.contains(&binding) => {
                                Ok(self.memo.binding_layouts.get(&binding).cloned())
                            }
                            None => Ok(fallback),
                        }
                    }
                    None => Ok(fallback),
                }
            }
            Op::History { cell, fallback, .. } => match fallback {
                Some(_) => Ok(layouts.first().cloned().flatten()),
                None => match cell {
                    HistoryCell::Input(input) => {
                        let bound = some!(self.bound(Query::Input(*input), need));
                        Ok(self.bound_layout(&bound)?)
                    }
                    HistoryCell::Let(_) | HistoryCell::State(_) => Err(None),
                },
            },
            Op::Native { native, .. } => {
                let decl = self
                    .vocabulary
                    .native(*native)
                    .ok_or(Internal::missed("native function"))?;
                Ok(native_layout(&decl.signature.result))
            }
            Op::Builtin(Builtin::Bytes, args) => match layouts.first().cloned().flatten() {
                Some(layout) => Ok(Some(layout)),
                None => {
                    let operand = *args.first().ok_or(Internal::missed("bytes operand"))?;
                    let sig = some!(self.sig(operand, need));
                    if sig.kind == Kind::Str && !sig.reactive {
                        match some!(self.cval(Query::Eval(operand, EvalTy::Str), need)) {
                            CVal::Str(text) => Ok(Some(Layout::fixed(vec![
                                u32::try_from(text.len()).unwrap_or(u32::MAX),
                            ]))),
                            _ => Err(None),
                        }
                    } else {
                        Err(Some(ShapeFault {
                            node,
                            code: Code::OperandType,
                            message: BYTES,
                        }))
                    }
                }
            },
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Arg(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Pick { .. }
            | Op::Convert(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::ToRead { .. } => Ok(None),
        };
        let certain = certain
            && match op {
                Op::History {
                    cell: HistoryCell::Let(_),
                    ..
                } => false,
                Op::History {
                    cell: HistoryCell::State(state),
                    ..
                } => self
                    .state_binding(*state)
                    .is_some_and(|binding| self.memo.settled.contains(&binding)),
                Op::Read(binding) => self.settled_read(*binding)?,
                _ => true,
            };
        Ok(Some(LayoutFound { shaped, certain }))
    }

    pub(in crate::semantics::analysis) fn state_binding(
        &self,
        state: StateId,
    ) -> Option<BindingId> {
        match self.memo.states.get(&state) {
            Some(BoundVal::Scalar(Bound::Cell(binding))) => Some(*binding),
            Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Array(_) | BoundVal::Error)
            | None => None,
        }
    }

    fn settled_read(&self, binding: BindingId) -> Result<bool, Internal> {
        Ok(
            !matches!(self.binding(binding)?.kind, BindingKind::State { .. })
                || self.memo.settled.contains(&binding),
        )
    }

    fn variable_layout(&self, binding: BindingId) -> Option<Layout> {
        self.memo
            .binding_layouts
            .get(&binding)
            .map(|layout| Layout::variable(layout.inner().to_vec()))
    }

    pub(in crate::semantics::analysis) fn chosen_arm(
        &self,
        op: &Op,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<usize>>, Internal> {
        match op {
            Op::If(condition, ..) => {
                let sig = some!(self.sig(*condition, need));
                if sig.reactive || sig.is_error() || sig.frame || sig.kind != Kind::Bool {
                    return Ok(Some(None));
                }
                Ok(Some(
                    match some!(self.cval(Query::Eval(*condition, EvalTy::Bool), need)) {
                        CVal::Bool(true) => Some(0),
                        CVal::Bool(false) => Some(1),
                        _ => None,
                    },
                ))
            }
            Op::Match {
                scrutinee,
                arms,
                otherwise,
            } => {
                let sig = some!(self.sig(*scrutinee, need));
                if sig.reactive || sig.is_error() || sig.frame {
                    return Ok(Some(None));
                }
                let Some(label_ty) = self.scrutinee_ty(*scrutinee, &sig) else {
                    return Ok(Some(None));
                };
                let value = some!(self.cval(Query::Eval(*scrutinee, label_ty), need));
                if value.is_error() {
                    return Ok(Some(None));
                }
                for (position, (label, _)) in arms.iter().enumerate() {
                    let label_value = some!(self.cval(Query::Eval(*label, label_ty), need));
                    if label_value.is_error() {
                        return Ok(Some(None));
                    }
                    if label_value == value {
                        return Ok(Some(Some(position)));
                    }
                }
                Ok(Some(otherwise.map(|_| arms.len())))
            }
            _ => Ok(Some(None)),
        }
    }

    fn choice_layout(&self, node: NodeId, layouts: &[Option<Layout>], reactive: bool) -> Shaped {
        let first = layouts.first().cloned().flatten();
        if layouts.iter().all(|layout| *layout == first) {
            return Ok(first);
        }
        let Some(inner) = first.as_ref().map(|layout| layout.inner().to_vec()) else {
            return Err(Some(ShapeFault {
                node,
                code: Code::TypeMismatch,
                message: BRANCHES,
            }));
        };
        let agree = layouts.iter().all(|layout| {
            layout
                .as_ref()
                .is_some_and(|layout| layout.inner() == inner.as_slice())
        });
        if agree && reactive {
            Ok(Some(Layout::variable(inner)))
        } else {
            Err(Some(ShapeFault {
                node,
                code: Code::TypeMismatch,
                message: BRANCHES,
            }))
        }
    }

    fn slice_bound_value(
        &self,
        bound: Option<(NodeId, bool)>,
        length: u32,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<Option<i128>>>, Internal> {
        let Some((node, from_end)) = bound else {
            return Ok(Some(Some(None)));
        };
        let sig = some!(self.sig(node, need));
        if sig.reactive || sig.frame || sig.is_error() || sig.kind != Kind::Int {
            return Ok(Some(None));
        }
        Ok(Some(
            match some!(self.cval(Query::Eval(node, EvalTy::Exact), need)) {
                CVal::Int(value) => Some(Some(if from_end {
                    i128::from(length) - value
                } else {
                    value
                })),
                CVal::F32(_)
                | CVal::F64(_)
                | CVal::Bool(_)
                | CVal::Enum(..)
                | CVal::Str(_)
                | CVal::Node(_)
                | CVal::Array(_)
                | CVal::Error => None,
            },
        ))
    }

    pub(in crate::semantics::analysis) fn bound_layout(
        &self,
        bound: &BoundVal,
    ) -> Result<Option<Layout>, Internal> {
        Ok(match bound {
            BoundVal::Array(array) => Some(Layout::fixed(array.0.lengths())),
            BoundVal::Scalar(Bound::Cell(binding)) => {
                self.memo.binding_layouts.get(binding).cloned()
            }
            BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error => None,
        })
    }

    pub(in crate::semantics::analysis) fn history_holds_array(
        &self,
        cell: HistoryCell,
    ) -> Result<bool, Internal> {
        Ok(match cell {
            HistoryCell::Input(input) => match self.memo.inputs.get(&input) {
                Some(BoundVal::Array(_)) => true,
                Some(BoundVal::Scalar(Bound::Cell(binding))) => {
                    self.memo.binding_layouts.contains_key(binding)
                }
                Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => false,
            },
            HistoryCell::Let(_) | HistoryCell::State(_) => false,
        })
    }

    pub(in crate::semantics::analysis) fn native_returns_array(
        &self,
        native: symphonyscript_core::NativeId,
    ) -> Result<bool, Internal> {
        let decl = self
            .vocabulary
            .native(native)
            .ok_or(Internal::missed("native function"))?;
        Ok(native_layout(&decl.signature.result).is_some())
    }
}

fn scalar_found() -> LayoutFound {
    LayoutFound {
        shaped: Ok(None),
        certain: true,
    }
}

fn fault(fault: Option<ShapeFault>) -> LayoutFound {
    LayoutFound {
        shaped: Err(fault),
        certain: true,
    }
}

const LENGTH: &str = "an array's length and capacity are below 2³¹";

const JOIN_ROWS: &str = "the rows of the joined arrays have different shapes";

const RAGGED: &str = "the elements of an array are arrays of one length, or all single values";

const BRANCHES: &str = "the branches hold arrays of different shapes";

const NOT_AN_ARRAY: &str = "`len` takes an array";

const BOUND_TYPE: &str = "a slice bound is an integer, not an array";

const BYTES: &str = "`bytes` takes a string or an integer array";
