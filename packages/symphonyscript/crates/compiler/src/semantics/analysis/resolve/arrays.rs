use crate::semantics::builtin::Builtin;
use symphonyscript_core::{BinaryOp, Scalar};

use super::annotation::Annotation;
use super::layout_found::LayoutFound;
use super::length_limit::LENGTH_LIMIT;
use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::bound::Bound;
use crate::semantics::analysis::bound_val::BoundVal;
use crate::semantics::analysis::c_val::CVal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::eval_ty::EvalTy;
use crate::semantics::analysis::fast_hash::{FastMap, FastSet};
use crate::semantics::analysis::history_cell::HistoryCell;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::layout::Layout;
use crate::semantics::analysis::let_ref::LetRef;
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, Loc, Number, SliceBound, Suffix};

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

    fn bare_leaves(&self, roots: &[NodeId]) -> Result<Vec<NodeId>, Internal> {
        let mut found = Vec::new();
        let mut seen = FastSet::default();
        let mut stack = roots.to_vec();
        while let Some(node) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::ArraysBareLeaves.step();
            if !seen.insert(node) {
                continue;
            }
            match &self.node(node)?.op {
                Op::Bare(_) => found.push(node),
                Op::Use(_) | Op::Read(_) => {}
                op => stack.extend(op.operands()),
            }
        }
        found.sort();
        Ok(found)
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
                for bare in self.bare_leaves(&array.elements)? {
                    self.unplaced_placeholder(bare)?;
                }
                self.typing
                    .measured_elements
                    .extend(array.elements.iter().copied());
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
            if !some!(self.sig(node, need)).is_error() {
                self.error(Code::NotAnArray, loc, scope, NOT_AN_ARRAY)?;
            }
            return Ok(Some(Val::Error));
        }
        if some!(self.sig(node, need)).frame && self.rests_on_recursion(node)? {
            return Ok(Some(self.scalar(
                Op::Builtin(Builtin::Len, vec![node]),
                loc,
                scope,
            )?));
        }
        let found = some!(self.layout_of(node, need)?);
        let certain = found.certain;
        let length = self.length_of_layout(node, found, (loc, scope))?;
        if matches!(length, Val::Scalar(measured) if !matches!(self.node(measured)?.op, Op::Builtin(..)))
        {
            self.memo.own_typed.push(node);
        }
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
        if !wants_bytes {
            return Ok(Some(val));
        }
        if let Some(bytes) = self.memo.string_bytes.get(&node) {
            return Ok(Some(Val::Scalar(*bytes)));
        }
        if self.reaches_choice(node)? {
            return Ok(Some(match some!(self.choice_bytes(node, need)?) {
                Some(bytes) => Val::Scalar(bytes),
                None => val,
            }));
        }
        if self.arrays.contains(&node) {
            return Ok(Some(val));
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

    pub(in crate::semantics::analysis) fn choice_bytes(
        &mut self,
        root: NodeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Option<NodeId>>, Internal> {
        let mut order = Vec::new();
        let mut seen = FastSet::default();
        let mut strings = FastSet::default();
        let mut stack = vec![(root, false)];
        while let Some((node, expanded)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::ArraysChoiceBytes.step();
            if expanded {
                order.push(node);
                continue;
            }
            if !seen.insert(node) {
                continue;
            }
            stack.push((node, true));
            let arms = match &self.node(node)?.op {
                Op::Use(inner) | Op::Chosen(inner) => vec![*inner],
                Op::If(_, then, otherwise) => vec![*then, *otherwise],
                Op::Match {
                    arms, otherwise, ..
                } => arms
                    .iter()
                    .map(|(_, value)| *value)
                    .chain(*otherwise)
                    .collect(),
                _ => {
                    let sig = some!(self.sig(node, need));
                    if sig.is_error() || (sig.kind == Kind::Str && sig.reactive) {
                        return Ok(Some(None));
                    }
                    if sig.kind == Kind::Str {
                        strings.insert(node);
                    }
                    Vec::new()
                }
            };
            stack.extend(arms.into_iter().map(|arm| (arm, false)));
        }
        if strings.is_empty() {
            return Ok(Some(None));
        }
        let mut bytes: FastMap<NodeId, NodeId> = FastMap::default();
        for node in order {
            let record = self.node(node)?.clone();
            let mapped = |inner: &NodeId| {
                bytes
                    .get(inner)
                    .copied()
                    .ok_or(Internal::missed("the bytes of an arm"))
            };
            let created = match &record.op {
                Op::Use(inner) | Op::Chosen(inner) => mapped(inner)?,
                Op::If(condition, then, otherwise) => self.push_node(
                    Op::If(*condition, mapped(then)?, mapped(otherwise)?),
                    record.loc,
                    record.scope,
                )?,
                Op::Match {
                    scrutinee,
                    arms,
                    otherwise,
                } => {
                    let arms = arms
                        .iter()
                        .map(|(label, value)| Ok((*label, mapped(value)?)))
                        .collect::<Result<Vec<_>, Internal>>()?;
                    let otherwise = otherwise.as_ref().map(mapped).transpose()?;
                    self.push_node(
                        Op::Match {
                            scrutinee: *scrutinee,
                            arms,
                            otherwise,
                        },
                        record.loc,
                        record.scope,
                    )?
                }
                _ if !strings.contains(&node) => node,
                _ => match self.memo.string_bytes.get(&node) {
                    Some(created) => *created,
                    None => {
                        let created = self.push_node(
                            Op::Builtin(Builtin::Bytes, vec![node]),
                            record.loc,
                            record.scope,
                        )?;
                        self.memo.string_bytes.insert(node, created);
                        created
                    }
                },
            };
            bytes.insert(node, created);
        }
        let created = bytes
            .get(&root)
            .copied()
            .ok_or(Internal::missed("the bytes of a choice"))?;
        self.memo.string_bytes.insert(root, created);
        Ok(Some(Some(created)))
    }

    pub(in crate::semantics::analysis) fn reaches_choice(
        &self,
        mut node: NodeId,
    ) -> Result<bool, Internal> {
        loop {
            match &self.node(node)?.op {
                Op::Use(inner) | Op::Chosen(inner) => node = *inner,
                Op::If(..) | Op::Match { .. } => return Ok(true),
                _ => return Ok(false),
            }
        }
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
        let index = self.push_node(Op::Binary(BinaryOp::Sub, length, offset), loc, scope)?;
        self.memo.from_end_indexes.insert(index);
        Ok(Some(index))
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
                Val::Array(_) if some!(self.placeholder_array(&val, need)?) => {
                    return Ok(Some(Val::Error));
                }
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
                if !some!(self.sig(*node, need)).is_error() {
                    self.error(
                        Code::NotAnArray,
                        self.hir.expr(base)?.loc,
                        scope,
                        "only an array is sliced",
                    )?;
                }
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
        let Some(whole) = self.boxed(&base_val, self.hir.expr(base)?.loc, scope)? else {
            return Ok(Some(Val::Error));
        };
        let sliced = self.scalar(
            Op::Slice {
                base: whole,
                lower,
                upper,
            },
            loc,
            scope,
        )?;
        for (bound, _) in lower.iter().chain(upper.iter()) {
            self.memo.eager_reads.push((*bound, sliced.nodes()));
        }
        Ok(Some(sliced))
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

    pub(in crate::semantics::analysis) fn slice_bound_value(
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
            match some!(self.untaken_cval(Query::Eval(node, EvalTy::Exact), need)) {
                CVal::Int(value) if i64::try_from(value).is_ok() => Some(Some(if from_end {
                    i128::from(length) - value
                } else {
                    value
                })),
                CVal::Int(_)
                | CVal::F32(_)
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

    pub(in crate::semantics::analysis) fn let_bound(
        &self,
        id: crate::semantics::ir::LetId,
    ) -> Option<&BoundVal> {
        self.memo
            .lets
            .get(&(self.global_of_let(id), LetRef::Top(id)))
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
            HistoryCell::State(state) => match self.memo.states.get(&state) {
                Some(BoundVal::Scalar(Bound::Cell(binding))) => {
                    self.memo.binding_layouts.contains_key(binding)
                        || self.arrays.contains(&self.binding(*binding)?.init)
                }
                None => true,
                Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Array(_) | BoundVal::Error) => {
                    false
                }
            },
            HistoryCell::Let(id) => match self.let_bound(id) {
                Some(BoundVal::Array(_)) => true,
                Some(BoundVal::Scalar(Bound::Cell(binding))) => {
                    self.memo.binding_layouts.contains_key(binding)
                        || self.arrays.contains(&self.binding(*binding)?.init)
                }
                Some(BoundVal::Scalar(Bound::Value(_)) | BoundVal::Error) | None => false,
            },
        })
    }

    pub(in crate::semantics::analysis) fn to_read_layout(
        &self,
        attrs: &[(
            crate::semantics::analysis::item_ref::ItemRef,
            symphonyscript_core::AttrId,
        )],
    ) -> Result<Option<Layout>, Internal> {
        let Some((item, attr)) = attrs.first() else {
            return Ok(None);
        };
        let decl = self
            .attr_decl(*item, *attr)?
            .ok_or(Internal::missed("attribute"))?;
        Ok(match decl.ty.capacity {
            Some(_) => Some(Layout::variable(decl.ty.lengths.clone())),
            None if decl.ty.lengths.is_empty() => None,
            None => Some(Layout::fixed(decl.ty.lengths.clone())),
        })
    }
}

const LENGTH: &str = "an array's length and capacity are below 2³¹";

const JOIN_ROWS: &str = "the rows of the joined arrays have different shapes";

pub(crate) const NOT_AN_ARRAY: &str = "`.size` reads the length of an array";

const BOUND_TYPE: &str = "a slice bound is an integer, not an array";
