use crate::semantics::builtin::Builtin;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::answers::Dimensions;
use crate::semantics::analysis::assumed::Assumed;
use crate::semantics::analysis::binding_kind::BindingKind;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{BindingId, NodeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::lengths_of::lengths_of;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::resolve::LENGTH_LIMIT;
use crate::semantics::analysis::resolve::LayoutFound;
use crate::semantics::analysis::resolve::Shaped;
use crate::semantics::analysis::resolve::native_layout;
use crate::semantics::analysis::settling::Settling;
use crate::semantics::analysis::shape_fault::ShapeFault;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{FnId, StateId};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn definite_array(
        &self,
        root: NodeId,
    ) -> Result<bool, Internal> {
        let mut leaves = vec![root];
        while let Some(mut node) = leaves.pop() {
            loop {
                self.memo.arm_walks.set(self.memo.arm_walks.get() + 1);
                if !self.arrays.contains(&node) {
                    return Ok(false);
                }
                match &self.node(node)?.op {
                    Op::Use(inner) | Op::Chosen(inner) | Op::Annotate(_, inner) => node = *inner,
                    Op::Read(binding) => {
                        if self.memo.binding_layouts.contains_key(binding) {
                            break;
                        }
                        node = self.binding(*binding)?.init;
                    }
                    Op::History { .. } if self.typing.open_reads.contains(&node) => {
                        if self.open_read_holds_array(node)? {
                            break;
                        }
                        return Ok(false);
                    }
                    Op::Join(..)
                    | Op::Slice { .. }
                    | Op::Array(_)
                    | Op::History { .. }
                    | Op::Stepped(_)
                    | Op::Native { .. }
                    | Op::Builtin(Builtin::Bytes, _) => break,
                    Op::If(..) | Op::Match { .. }
                        if self
                            .memo
                            .choice_arrays
                            .get(&node)
                            .is_some_and(|(definite, _)| *definite) =>
                    {
                        break;
                    }
                    Op::If(_, then, otherwise) => {
                        leaves.extend([*then, *otherwise]);
                        break;
                    }
                    Op::Match {
                        arms, otherwise, ..
                    } => {
                        leaves.extend(arms.iter().map(|(_, value)| *value));
                        leaves.extend(otherwise.iter().copied());
                        break;
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
                    | Op::Convert(..)
                    | Op::Builtin(..)
                    | Op::Recursive { .. } => return Ok(false),
                    Op::Element { .. } | Op::For(_) | Op::Slot(_) => match self.dimensions(node) {
                        Dimensions::Known(_) => break,
                        Dimensions::Unknown => return Ok(false),
                    },
                    Op::ToRead { attrs, .. } => {
                        if self.to_read_layout(attrs)?.is_none() {
                            return Ok(false);
                        }
                        break;
                    }
                }
            }
        }
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn record_choice_arrays(
        &mut self,
        choice: NodeId,
        arms: &[NodeId],
    ) -> Result<(), Internal> {
        let mut definite = true;
        let mut rests = false;
        for arm in arms {
            definite &= self.definite_array(*arm)?;
            rests |= self.rests_on_recursion(*arm)?;
        }
        self.memo.choice_arrays.insert(choice, (definite, rests));
        Ok(())
    }

    pub(in crate::semantics::analysis) fn reaches_recursion(
        &self,
        root: NodeId,
    ) -> Result<bool, Internal> {
        let mut stack = vec![root];
        let mut seen = FastSet::default();
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::RunReachesRecursion.step();
            if !seen.insert(node) {
                continue;
            }
            let op = &self.node(node)?.op;
            match op {
                Op::Recursive { .. } => return Ok(true),
                Op::Use(_) | Op::Read(_) => {}
                Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Bare(_)
                | Op::Pi
                | Op::NodeRef(_)
                | Op::Chosen(_)
                | Op::Arg(_)
                | Op::Slot(_)
                | Op::For(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::If(..)
                | Op::Match { .. }
                | Op::Pick { .. }
                | Op::Element { .. }
                | Op::Array(_)
                | Op::Convert(..)
                | Op::Annotate(..)
                | Op::Builtin(..)
                | Op::ToRead { .. }
                | Op::Join(..)
                | Op::Slice { .. }
                | Op::History { .. }
                | Op::Stepped(_)
                | Op::Native { .. } => stack.extend(op.operands()),
            }
        }
        Ok(false)
    }

    pub(in crate::semantics::analysis) fn rests_on_recursion(
        &self,
        root: NodeId,
    ) -> Result<bool, Internal> {
        let mut stack = vec![root];
        let mut seen = FastSet::default();
        while let Some(node) = stack.pop() {
            self.memo.arm_walks.set(self.memo.arm_walks.get() + 1);
            if !self.arrays.contains(&node) || !seen.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::Recursive { .. } => return Ok(true),
                Op::Binary(..) => {
                    if self.memo.deferred_sums.contains(&node) {
                        return Ok(true);
                    }
                }
                Op::Use(inner)
                | Op::Chosen(inner)
                | Op::Annotate(_, inner)
                | Op::Element { base: inner, .. }
                | Op::Slice { base: inner, .. } => stack.push(*inner),
                Op::Join(lhs, rhs) => stack.extend([*lhs, *rhs]),
                Op::Builtin(Builtin::Bytes, args) => stack.extend(args.iter().copied()),
                Op::Read(binding) => {
                    if !self.memo.binding_layouts.contains_key(binding) {
                        stack.push(self.binding(*binding)?.init);
                    }
                }
                Op::If(..) | Op::Match { .. }
                    if self
                        .memo
                        .choice_arrays
                        .get(&node)
                        .is_some_and(|(_, rests)| !*rests) => {}
                Op::If(_, then, otherwise) => stack.extend([*then, *otherwise]),
                Op::Match {
                    arms, otherwise, ..
                } => {
                    stack.extend(arms.iter().map(|(_, value)| *value));
                    stack.extend(otherwise.iter().copied());
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
                | Op::Slot(_)
                | Op::For(_)
                | Op::Unary(..)
                | Op::Pick { .. }
                | Op::Array(_)
                | Op::Convert(..)
                | Op::Builtin(..)
                | Op::ToRead { .. }
                | Op::History { .. }
                | Op::Stepped(_)
                | Op::Native { .. } => {}
            }
        }
        Ok(false)
    }

    pub(in crate::semantics::analysis) fn layout_of(
        &mut self,
        root: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<LayoutFound>, Internal> {
        self.layout_in(root, None, need)
    }

    pub(in crate::semantics::analysis) fn known_lengths(
        &self,
        root: NodeId,
    ) -> Result<Option<Vec<Option<u32>>>, Internal> {
        self.lengths_on_any_path(root, false)
    }

    fn lengths_on_any_path(
        &self,
        root: NodeId,
        in_template: bool,
    ) -> Result<Option<Vec<Option<u32>>>, Internal> {
        let mut steps = Vec::new();
        let mut alternatives: Vec<(NodeId, usize)> = Vec::new();
        let mut seen = FastSet::default();
        let mut node = root;
        let mut lengths = loop {
            let found = 'path: loop {
                if !self.arrays.contains(&node) {
                    break 'path Some(Vec::new());
                }
                if in_template && !seen.insert(node) {
                    break 'path None;
                }
                if let Some(found) = self.cached_layout(node, None) {
                    match found.shaped {
                        Ok(Some(layout)) => break 'path Some(open_lengths(&layout)),
                        Ok(None) => break 'path Some(Vec::new()),
                        Err(_) => return Ok(None),
                    }
                }
                match &self.node(node)?.op {
                    Op::Use(inner) | Op::Chosen(inner) | Op::Annotate(_, inner) => node = *inner,
                    Op::Read(binding) => match self.memo.binding_layouts.get(binding) {
                        Some(layout) => break 'path Some(open_lengths(layout)),
                        None => node = self.binding(*binding)?.init,
                    },
                    Op::Element { base, .. } => {
                        steps.push(Step::Indexed);
                        node = *base;
                    }
                    Op::Slice { base, .. } | Op::Join(base, _) => {
                        steps.push(Step::Resized);
                        node = *base;
                    }
                    Op::Array(elements) => {
                        let Some(first) = elements.first() else {
                            break 'path None;
                        };
                        steps.push(Step::Wrapped(
                            u32::try_from(elements.len()).unwrap_or(u32::MAX),
                        ));
                        node = *first;
                    }
                    Op::If(_, then, otherwise) => match self.memo.choice_lengths.get(&node) {
                        Some(Some(lengths)) => break 'path Some(lengths.clone()),
                        Some(None) | None if in_template => {
                            alternatives.push((*otherwise, steps.len()));
                            node = *then;
                        }
                        Some(None) | None => break 'path None,
                    },
                    Op::Match {
                        arms, otherwise, ..
                    } => match self.memo.choice_lengths.get(&node) {
                        Some(Some(lengths)) => break 'path Some(lengths.clone()),
                        Some(None) | None if in_template => {
                            let mut values = arms.iter().map(|(_, value)| *value).chain(*otherwise);
                            let Some(first) = values.next() else {
                                break 'path None;
                            };
                            alternatives.extend(values.map(|value| (value, steps.len())));
                            node = first;
                        }
                        Some(None) | None => break 'path None,
                    },
                    Op::Builtin(Builtin::Bytes, args) => match args.first() {
                        Some(operand) if self.arrays.contains(operand) => node = *operand,
                        Some(_) => break 'path Some(vec![None]),
                        None => break 'path None,
                    },
                    Op::Native { native, .. } => {
                        let decl = self
                            .vocabulary
                            .native(*native)
                            .ok_or(Internal::missed("native function"))?;
                        break 'path Some(
                            native_layout(&decl.signature.result)
                                .as_ref()
                                .map(open_lengths)
                                .unwrap_or_default(),
                        );
                    }
                    Op::ToRead { attrs, .. } => {
                        break 'path Some(
                            self.to_read_layout(attrs)?
                                .as_ref()
                                .map(open_lengths)
                                .unwrap_or_default(),
                        );
                    }
                    Op::History { .. } => match self.history_bindings(node)?.as_slice() {
                        [binding] if self.settled_layout(*binding)? => {
                            match self.memo.binding_layouts.get(binding) {
                                Some(layout) => break 'path Some(open_lengths(layout)),
                                None => break 'path None,
                            }
                        }
                        _ => break 'path None,
                    },
                    Op::Slot(slot) => {
                        break 'path Some(
                            self.slot(*slot)?
                                .layout
                                .as_ref()
                                .map(open_lengths)
                                .unwrap_or_default(),
                        );
                    }
                    Op::For(for_node) => {
                        break 'path Some(
                            self.slot(for_node.accumulator)?
                                .layout
                                .as_ref()
                                .map(open_lengths)
                                .unwrap_or_default(),
                        );
                    }
                    Op::Recursive { .. } if in_template => break 'path None,
                    Op::Recursive { function, args } => {
                        break 'path self.recursive_lengths(*function, args)?;
                    }
                    Op::Number(..)
                    | Op::Const(_)
                    | Op::Bool(_)
                    | Op::Stepped(_)
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
                    | Op::Builtin(..) => break 'path None,
                }
            };
            if let Some(found) = found {
                break found;
            }
            let Some((alternative, depth)) = alternatives.pop() else {
                return Ok(None);
            };
            steps.truncate(depth);
            node = alternative;
        };
        for step in steps.into_iter().rev() {
            match step {
                Step::Indexed => {
                    if lengths.pop().is_none() {
                        return Ok(None);
                    }
                }
                Step::Resized => match lengths.last_mut() {
                    Some(outer) => *outer = None,
                    None => return Ok(None),
                },
                Step::Wrapped(length) => lengths.push(Some(length)),
            }
        }
        Ok(Some(lengths))
    }

    fn recursive_lengths(
        &self,
        function: FnId,
        args: &[Option<Val>],
    ) -> Result<Option<Vec<Option<u32>>>, Internal> {
        let typed = args
            .iter()
            .flatten()
            .flat_map(Val::nodes)
            .all(|node| self.memo.sigs.contains_key(&node));
        if !typed {
            return Ok(None);
        }
        let Some((key, shapes)) = self.family_key(function, args, &mut Vec::new())? else {
            return Ok(None);
        };
        if self.memo.family_shapes.get(&key) != Some(&shapes) {
            return Ok(None);
        }
        let Some(&scope) = self
            .memo
            .family_scopes
            .get(&key)
            .and_then(|scopes| scopes.last())
        else {
            return Ok(None);
        };
        let body = self.hir.fn_decl(function)?.value;
        let root = match self.memo.exprs.get(&(body, scope)) {
            Some(Val::Scalar(node)) => *node,
            Some(Val::Array(array)) => {
                let mut lengths: Vec<Option<u32>> =
                    array.shape.lengths().into_iter().map(Some).collect();
                if let Some(outer) = lengths.last_mut() {
                    *outer = None;
                }
                return Ok(Some(lengths));
            }
            Some(Val::Error) | None => return Ok(None),
        };
        let Some(mut lengths) = self.lengths_on_any_path(root, true)? else {
            return Ok(None);
        };
        if let Some(outer) = lengths.last_mut() {
            *outer = None;
        }
        Ok(Some(lengths))
    }

    fn settled_layout(&self, binding: BindingId) -> Result<bool, Internal> {
        Ok(match self.binding(binding)?.kind {
            BindingKind::State { .. } => self.memo.settled.contains(&binding),
            BindingKind::Input { .. }
            | BindingKind::Let
            | BindingKind::Param
            | BindingKind::Attr { .. } => true,
        })
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
            Op::History { .. }
            | Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Stepped(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Use(_)
            | Op::Chosen(_)
            | Op::Arg(_)
            | Op::Slot(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::If(..)
            | Op::Match { .. }
            | Op::Pick { .. }
            | Op::Element { .. }
            | Op::Array(_)
            | Op::Convert(..)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::Native { .. } => None,
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
        let mut active: FastSet<NodeId> = FastSet::default();
        let mut expanded: FastMap<NodeId, Vec<NodeId>> = FastMap::default();
        let mut missing = false;
        while let Some((node, ready)) = stack.pop() {
            self.memo.layout_visits += 1;
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
            if !ready {
                if let Op::Slice { lower, upper, .. } = op {
                    self.slice_bound_value(lower, 0, need)?;
                    self.slice_bound_value(upper, 0, need)?;
                }
                let mut parts = self.layout_parts(&op, assumed)?;
                if parts.iter().any(|part| active.contains(part)) {
                    if let Op::Read(binding) = op {
                        found.insert(
                            node,
                            LayoutFound {
                                shaped: Ok(self.variable_layout(binding)),
                                certain: false,
                            },
                        );
                        continue;
                    }
                    parts.retain(|part| !active.contains(part));
                }
                active.insert(node);
                stack.push((node, true));
                stack.extend(parts.iter().map(|part| (*part, false)));
                expanded.insert(node, parts);
                continue;
            }
            active.remove(&node);
            let parts = expanded.remove(&node).unwrap_or_default();
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
            Op::History { cell, fallback, .. } => {
                let mut parts: Vec<NodeId> = fallback.iter().copied().collect();
                if let HistoryCell::Let(id) = cell
                    && let Some(BoundVal::Scalar(Bound::Cell(binding))) = self.let_bound(*id)
                    && !self.memo.binding_layouts.contains_key(binding)
                {
                    parts.push(self.binding(*binding)?.init);
                }
                if let (HistoryCell::State(state), None) = (cell, fallback)
                    && let Some(binding) = self.state_binding(*state)
                {
                    let member = assumed.and_then(|assumed| assumed.members.get(&binding));
                    let known = member == Some(&true)
                        || (member.is_none() && self.memo.binding_layouts.contains_key(&binding));
                    if !known {
                        parts.push(self.binding(binding)?.init);
                    }
                }
                parts
            }
            Op::Builtin(Builtin::Bytes, args) => args.clone(),
            Op::Number(..)
            | Op::Const(_)
            | Op::Bool(_)
            | Op::Stepped(_)
            | Op::Str(_)
            | Op::Concat(_)
            | Op::Variant(..)
            | Op::Bare(_)
            | Op::Pi
            | Op::NodeRef(_)
            | Op::Arg(_)
            | Op::Slot(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Pick { .. }
            | Op::Convert(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
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
            Op::If(..) | Op::Match { .. } => match some!(self.chosen_arm(op, need)?) {
                Some(_) if !arms_agree(&layouts) => Err(Some(ShapeFault {
                    node,
                    code: Code::TypeMismatch,
                    message: BRANCHES,
                })),
                Some(arm) => Ok(layouts.get(arm).cloned().flatten()),
                None => self.choice_layout(node, &layouts),
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
            Op::History {
                cell: HistoryCell::Let(id),
                ..
            } => match self.let_bound(*id) {
                Some(BoundVal::Scalar(Bound::Cell(binding))) => {
                    match self.memo.binding_layouts.get(binding) {
                        Some(layout) => Ok(Some(layout.clone())),
                        None => Ok(layouts.get(1).or(layouts.first()).cloned().flatten()),
                    }
                }
                Some(bound @ BoundVal::Array(_)) => Ok(self.bound_layout(bound)?),
                Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => {
                    Ok(layouts.first().cloned().flatten())
                }
            },
            Op::History { cell, .. } => match cell {
                HistoryCell::Input(input) => {
                    let bound = some!(self.bound(Query::Input(*input), need));
                    Ok(self.bound_layout(&bound)?)
                }
                HistoryCell::State(state) => match self.state_binding(*state) {
                    Some(binding) => {
                        match assumed.and_then(|assumed| assumed.members.get(&binding)) {
                            Some(true) => Ok(self.variable_layout(binding)),
                            Some(false) => Ok(layouts.first().cloned().flatten()),
                            None => match self.memo.binding_layouts.get(&binding) {
                                Some(layout) => Ok(Some(layout.clone())),
                                None => Ok(layouts.first().cloned().flatten()),
                            },
                        }
                    }
                    None => Ok(layouts.first().cloned().flatten()),
                },
                HistoryCell::Let(_) => Ok(layouts.first().cloned().flatten()),
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
                        match some!(self.untaken_cval(Query::Eval(operand, EvalTy::Str), need)) {
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
            | Op::Stepped(_)
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
            | Op::Recursive { .. } => Ok(None),
            Op::Slot(slot) => Ok(self.slot(*slot)?.layout.clone()),
            Op::For(for_node) => Ok(self.slot(for_node.accumulator)?.layout.clone()),
            Op::ToRead { attrs, .. } => Ok(self.to_read_layout(attrs)?),
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
                Op::History { .. }
                | Op::Number(..)
                | Op::Const(_)
                | Op::Bool(_)
                | Op::Stepped(_)
                | Op::Str(_)
                | Op::Concat(_)
                | Op::Variant(..)
                | Op::Bare(_)
                | Op::Pi
                | Op::NodeRef(_)
                | Op::Use(_)
                | Op::Chosen(_)
                | Op::Arg(_)
                | Op::Slot(_)
                | Op::Unary(..)
                | Op::Binary(..)
                | Op::If(..)
                | Op::Match { .. }
                | Op::Pick { .. }
                | Op::Element { .. }
                | Op::Array(_)
                | Op::Convert(..)
                | Op::Annotate(..)
                | Op::Builtin(..)
                | Op::Recursive { .. }
                | Op::For(_)
                | Op::ToRead { .. }
                | Op::Join(..)
                | Op::Slice { .. }
                | Op::Native { .. } => true,
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

    pub(in crate::semantics::analysis) fn open_read_holds_array(
        &self,
        node: NodeId,
    ) -> Result<bool, Internal> {
        let Op::History {
            cell: HistoryCell::State(state),
            ..
        } = self.node(node)?.op
        else {
            return Ok(false);
        };
        Ok(self.state_binding(state).is_some_and(|binding| {
            self.memo.settled.contains(&binding) && self.memo.binding_layouts.contains_key(&binding)
        }))
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
                    match some!(self.untaken_cval(Query::Eval(*condition, EvalTy::Bool), need)) {
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
                let labels: Vec<NodeId> = arms.iter().map(|(label, _)| *label).collect();
                let Some(label_ty) = self.choice_ty(*scrutinee, &sig, &labels)? else {
                    return Ok(Some(None));
                };
                let value = some!(self.untaken_cval(Query::Eval(*scrutinee, label_ty), need));
                if value.is_error() {
                    return Ok(Some(None));
                }
                for (position, (label, _)) in arms.iter().enumerate() {
                    let label_value = some!(self.untaken_cval(Query::Eval(*label, label_ty), need));
                    if label_value.is_error() {
                        return Ok(Some(None));
                    }
                    if label_value == value {
                        return Ok(Some(Some(position)));
                    }
                }
                Ok(Some(otherwise.map(|_| arms.len())))
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
            | Op::Read(_)
            | Op::Use(_)
            | Op::Chosen(_)
            | Op::Arg(_)
            | Op::Slot(_)
            | Op::Unary(..)
            | Op::Binary(..)
            | Op::Pick { .. }
            | Op::Element { .. }
            | Op::Array(_)
            | Op::Convert(..)
            | Op::Annotate(..)
            | Op::Builtin(..)
            | Op::Recursive { .. }
            | Op::For(_)
            | Op::ToRead { .. }
            | Op::Join(..)
            | Op::Slice { .. }
            | Op::History { .. }
            | Op::Stepped(_)
            | Op::Native { .. } => Ok(Some(None)),
        }
    }

    fn choice_layout(&self, node: NodeId, layouts: &[Option<Layout>]) -> Shaped {
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
        if agree {
            Ok(Some(Layout::variable(inner)))
        } else {
            Err(Some(ShapeFault {
                node,
                code: Code::TypeMismatch,
                message: BRANCHES,
            }))
        }
    }
}

enum Step {
    Indexed,
    Resized,
    Wrapped(u32),
}

fn open_lengths(layout: &Layout) -> Vec<Option<u32>> {
    let mut lengths: Vec<Option<u32>> = layout.inner().iter().copied().map(Some).collect();
    lengths.push(layout.outer());
    lengths
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

fn arms_agree(layouts: &[Option<Layout>]) -> bool {
    let depth =
        |layout: &Option<Layout>| layout.as_ref().map_or(0, |layout| layout.inner().len() + 1);
    let first = layouts.first().map_or(0, depth);
    layouts.iter().all(|layout| depth(layout) == first)
}

const LENGTH: &str = "an array's length and capacity are below 2³¹";

const JOIN_ROWS: &str = "the rows of the joined arrays have different shapes";

const RAGGED: &str = "the elements of an array are arrays of one length, or all single values";

const BRANCHES: &str = "the branches hold arrays of different shapes";
const BYTES: &str = "`bytes` takes a string or an integer array";
