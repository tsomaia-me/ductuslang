use symphonyscript_core::Value;

use crate::diagnostics::Code;
use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::op::Op;
use crate::semantics::analysis::piece::Piece;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::shape::Shape;
use crate::semantics::analysis::shape_need::ShapeNeed;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Loc, StrPart};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_expr(
        &mut self,
        expr: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<bool, Internal> {
        let hir = self.hir;
        let node = hir.expr(expr)?;
        let loc = node.loc;
        let val = match &node.kind {
            ExprKind::Error | ExprKind::To => Some(Val::Error),
            ExprKind::Number(number, suffix) => {
                Some(self.scalar(Op::Number(number.clone(), *suffix), loc, scope)?)
            }
            ExprKind::Bool(value) => Some(self.scalar(Op::Bool(*value), loc, scope)?),
            ExprKind::Str(parts) => self.string(parts, loc, scope, need)?,
            ExprKind::Name(name) => self.lookup(expr, name, loc, scope, need)?,
            ExprKind::Variant {
                enumeration,
                variant,
            } => Some(self.variant(enumeration, variant, loc, scope)?),
            ExprKind::Convert(target, operand) => {
                match self.val(Query::Expr(*operand, scope), need) {
                    Some(operand) => {
                        Some(self.unary_val(operand, loc, scope, |operand| {
                            Op::Convert(*target, operand)
                        })?)
                    }
                    None => None,
                }
            }
            ExprKind::Array(elements) => self.array(expr, elements, loc, scope, need)?,
            ExprKind::Field(base, name) => self.field(*base, name, loc, scope, need)?,
            ExprKind::Index(base, index) => self.index(*base, *index, loc, scope, need)?,
            ExprKind::Call(callee, args) => self.call(*callee, args, expr, loc, scope, need)?,
            ExprKind::Unary(op, operand) => match self.val(Query::Expr(*operand, scope), need) {
                Some(operand) => {
                    Some(self.unary_val(operand, loc, scope, |operand| Op::Unary(*op, operand))?)
                }
                None => None,
            },
            ExprKind::Binary(op, lhs, rhs) => {
                let (lhs, rhs) = self.siblings(*lhs, *rhs, scope, need)?;
                match (lhs, rhs) {
                    (Some(lhs), Some(rhs)) => Some(self.binary_val(*op, lhs, rhs, loc, scope)?),
                    _ => None,
                }
            }
            ExprKind::If(condition, then, otherwise) => {
                let condition = self.val(Query::Expr(*condition, scope), need);
                let (then_expr, otherwise_expr) = (*then, *otherwise);
                let (then, otherwise) = self.siblings(then_expr, otherwise_expr, scope, need)?;
                let then = self.held(then_expr, then, scope, need)?;
                let otherwise = self.held(otherwise_expr, otherwise, scope, need)?;
                match (condition, then, otherwise) {
                    (Some(condition), Some(then), Some(otherwise)) => {
                        self.if_val(condition, then, otherwise, (loc, scope), need)?
                    }
                    _ => None,
                }
            }
            ExprKind::Match {
                scrutinee,
                arms,
                otherwise,
            } => self.match_val(*scrutinee, arms, *otherwise, loc, scope, need)?,
        };
        let Some(val) = val else {
            return Ok(false);
        };
        self.memo.exprs.insert((expr, scope), val);
        Ok(true)
    }

    pub(in crate::semantics::analysis) fn scalar(
        &mut self,
        op: Op,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        Ok(Val::Scalar(self.push_node(op, loc, scope)?))
    }

    fn string(
        &mut self,
        parts: &[StrPart],
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let mut values = Vec::new();
        for part in parts {
            if let StrPart::Hole(expr) = part {
                values.push((*expr, self.val(Query::Expr(*expr, scope), need)));
            }
        }
        if values.iter().any(|(_, value)| value.is_none()) {
            return Ok(None);
        }
        if values.is_empty() {
            let text = parts
                .iter()
                .map(|part| match part {
                    StrPart::Text(text) => text.as_str(),
                    StrPart::Hole(_) => "",
                })
                .collect::<String>();
            return Ok(Some(self.scalar(Op::Str(text), loc, scope)?));
        }
        let mut holes = values.into_iter();
        let mut pieces = Vec::new();
        for part in parts {
            match part {
                StrPart::Text(text) => pieces.push(Piece::Text(text.clone())),
                StrPart::Hole(_) => match holes.next() {
                    Some((_, Some(Val::Scalar(node)))) => {
                        self.single_operand(
                            &Val::Scalar(node),
                            Code::InterpolationType,
                            "an interpolation holds a single value, not an array",
                        );
                        pieces.push(Piece::Value(node));
                    }
                    Some((hole, Some(Val::Array(_)))) => {
                        let hole_loc = self.hir.expr(hole)?.loc;
                        self.error(
                            Code::InterpolationType,
                            hole_loc,
                            scope,
                            "an interpolation holds a single value, not an array",
                        )?;
                        return Ok(Some(Val::Error));
                    }
                    Some((_, Some(Val::Error) | None)) => return Ok(Some(Val::Error)),
                    None => return Err(Internal::missed("interpolation")),
                },
            }
        }
        Ok(Some(self.scalar(Op::Concat(pieces), loc, scope)?))
    }

    fn variant(
        &mut self,
        enumeration: &crate::semantics::ir::Ident,
        variant: &crate::semantics::ir::Ident,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        let Some(id) = self.vocabulary.enum_named(&enumeration.text) else {
            self.error(
                Code::UnknownVariant,
                enumeration.loc,
                scope,
                format!("`{}` is not an enum of the vocabulary", enumeration.text),
            )?;
            return Ok(Val::Error);
        };
        Ok(match self.variants.position(id, &variant.text)? {
            Some(index) => self.scalar(Op::Variant(id, index), loc, scope)?,
            None => {
                self.error(
                    Code::UnknownVariant,
                    variant.loc,
                    scope,
                    format!(
                        "the enum `{}` has no variant `{}`",
                        enumeration.text, variant.text
                    ),
                )?;
                Val::Error
            }
        })
    }

    pub(in crate::semantics::analysis) fn unary_val(
        &mut self,
        operand: Val,
        loc: Loc,
        scope: ScopeId,
        op: impl Fn(NodeId) -> Op,
    ) -> Result<Val, Internal> {
        self.single_operand(&operand, Code::ArrayOperand, OPERAND);
        Ok(match operand {
            Val::Scalar(operand) => self.scalar(op(operand), loc, scope)?,
            Val::Array(_) => {
                self.error(
                    Code::ArrayOperand,
                    loc,
                    scope,
                    "operators and conversions apply to single values, not arrays",
                )?;
                Val::Error
            }
            Val::Error => Val::Error,
        })
    }

    fn binary_val(
        &mut self,
        op: symphonyscript_core::BinaryOp,
        lhs: Val,
        rhs: Val,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        self.single_operand(&lhs, Code::ArrayOperand, OPERAND);
        self.single_operand(&rhs, Code::ArrayOperand, OPERAND);
        Ok(match (lhs, rhs) {
            (Val::Scalar(lhs), Val::Scalar(rhs)) => {
                self.scalar(Op::Binary(op, lhs, rhs), loc, scope)?
            }
            (Val::Error, _) | (_, Val::Error) => Val::Error,
            _ => {
                self.error(
                    Code::ArrayOperand,
                    loc,
                    scope,
                    "operators apply to single values, not arrays",
                )?;
                Val::Error
            }
        })
    }

    fn if_val(
        &mut self,
        condition: Val,
        then: Val,
        otherwise: Val,
        (loc, scope): (Loc, ScopeId),
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        self.single_operand(
            &condition,
            Code::ConditionType,
            "the condition of an `if` is a `bool`, not an array",
        );
        let condition = match condition {
            Val::Scalar(condition) => condition,
            Val::Array(_) => {
                self.error(
                    Code::ConditionType,
                    loc,
                    scope,
                    "the condition of an `if` is a `bool`, not an array",
                )?;
                return Ok(Some(Val::Error));
            }
            Val::Error => return Ok(Some(Val::Error)),
        };
        let vals =
            match some!(self.branches(vec![then, otherwise], condition, (loc, scope), need)?) {
                Branches::Whole(nodes) => {
                    let &[then, otherwise] = nodes.as_slice() else {
                        return Ok(Some(Val::Error));
                    };
                    return Ok(Some(self.scalar(
                        Op::If(condition, then, otherwise),
                        loc,
                        scope,
                    )?));
                }
                Branches::Each(vals) => vals,
            };
        let Some((shape, pairs)) = self.zip_shapes(&vals, loc, scope)? else {
            return Ok(Some(Val::Error));
        };
        let elements: Vec<NodeId> = pairs
            .into_iter()
            .map(|pair| {
                let &[then, otherwise] = pair.as_slice() else {
                    return Err(Internal::missed("branch"));
                };
                self.push_node(Op::If(condition, then, otherwise), loc, scope)
            })
            .collect::<Result<_, _>>()?;
        Ok(Some(self.shaped(shape, elements)))
    }

    fn branches(
        &mut self,
        vals: Vec<Val>,
        selector: NodeId,
        (loc, scope): (Loc, ScopeId),
        need: &mut Vec<Query>,
    ) -> Result<Option<Branches>, Internal> {
        if vals.iter().any(|val| matches!(val, Val::Error)) {
            return Ok(Some(Branches::Each(vals)));
        }
        let opaque = vals.iter().any(|val| self.holds_array(val));
        let first = vals.first().map(Val::shape).unwrap_or_default();
        let differ = vals.iter().any(|val| val.shape() != first);
        let recursive_body = self.template_of(scope)?.is_some();
        let irregular = opaque || (differ && recursive_body);
        if !irregular {
            return Ok(Some(Branches::Each(vals)));
        }
        if !some!(self.sig(selector, need)).reactive {
            let mut nodes = Vec::with_capacity(vals.len());
            for val in &vals {
                nodes.extend(self.boxed(val, loc, scope)?);
            }
            return Ok(Some(Branches::Whole(nodes)));
        }
        let Some(shape) = vals
            .iter()
            .find(|val| !self.holds_array(val))
            .map(Val::shape)
        else {
            return Ok(Some(Branches::Each(vals)));
        };
        let mut each = Vec::new();
        for val in vals {
            let Val::Scalar(node) = val else {
                each.push(val);
                continue;
            };
            if !self.arrays.contains(&node) {
                each.push(val);
                continue;
            }
            let nodes = self.projected(node, &shape, (Code::TypeMismatch, BRANCHES), loc, scope)?;
            each.push(self.shaped(shape.clone(), nodes));
        }
        Ok(Some(Branches::Each(each)))
    }

    pub(in crate::semantics::analysis) fn require_shape(
        &mut self,
        node: NodeId,
        lengths: Vec<u32>,
        code: Code,
        message: &'static str,
    ) {
        self.memo.shapes.entry(node).or_insert(ShapeNeed {
            lengths,
            code,
            message,
        });
    }

    pub(in crate::semantics::analysis) fn single_operand(
        &mut self,
        val: &Val,
        code: Code,
        message: &'static str,
    ) {
        if let Val::Scalar(node) = val
            && self.arrays.contains(node)
        {
            self.require_shape(*node, Vec::new(), code, message);
        }
    }

    pub(in crate::semantics::analysis) fn projected(
        &mut self,
        node: NodeId,
        shape: &Shape,
        need: (Code, &'static str),
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Vec<NodeId>, Internal> {
        if let Some(nodes) = self.memo.projections.get(&node) {
            return Ok(nodes.clone());
        }
        let lengths = shape.lengths();
        self.require_shape(node, lengths.clone(), need.0, need.1);
        let mut level = vec![node];
        for length in lengths.iter().rev() {
            let mut next = Vec::new();
            for parent in &level {
                for position in 0..*length {
                    let index =
                        self.push_node(Op::Const(Value::I64(i64::from(position))), loc, scope)?;
                    next.push(self.push_node(
                        Op::Element {
                            base: *parent,
                            index,
                        },
                        loc,
                        scope,
                    )?);
                }
            }
            level = next;
        }
        self.memo.projections.insert(node, level.clone());
        Ok(level)
    }

    pub(in crate::semantics::analysis) fn boxed_body(
        &mut self,
        body: ExprId,
        scope: ScopeId,
        val: &Val,
    ) -> Result<Option<NodeId>, Internal> {
        if let Some(node) = self.memo.boxes.get(&(body, scope)) {
            return Ok(Some(*node));
        }
        let node = some!(self.boxed(val, self.hir.expr(body)?.loc, scope)?);
        if matches!(val, Val::Array(_)) {
            self.memo.boxes.insert((body, scope), node);
        }
        Ok(Some(node))
    }

    pub(in crate::semantics::analysis) fn boxed(
        &mut self,
        val: &Val,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Option<NodeId>, Internal> {
        let array = match val {
            Val::Scalar(node) => return Ok(Some(*node)),
            Val::Error => return Ok(None),
            Val::Array(array) => array,
        };
        let mut level = array.elements.clone();
        for length in array.shape.lengths() {
            let width = usize::try_from(length).unwrap_or(1).max(1);
            level = level
                .chunks(width)
                .map(|chunk| self.push_node(Op::Array(chunk.to_vec()), loc, scope))
                .collect::<Result<_, _>>()?;
        }
        Ok(level.first().copied())
    }

    pub(in crate::semantics::analysis) fn shaped(
        &mut self,
        shape: Shape,
        elements: Vec<NodeId>,
    ) -> Val {
        if shape.is_scalar() {
            match elements.first() {
                Some(node) => Val::Scalar(*node),
                None => Val::Error,
            }
        } else {
            Val::array(shape, elements)
        }
    }

    pub(in crate::semantics::analysis) fn zip_shapes(
        &mut self,
        vals: &[Val],
        loc: Loc,
        scope: ScopeId,
    ) -> Result<Option<Zipped>, Internal> {
        if vals.iter().any(|val| matches!(val, Val::Error)) {
            return Ok(None);
        }
        let shape = vals.first().map(Val::shape).unwrap_or_default();
        if vals.iter().any(|val| val.shape() != shape) {
            self.error(Code::TypeMismatch, loc, scope, BRANCHES)?;
            return Ok(None);
        }
        let columns: Vec<Vec<NodeId>> = vals.iter().map(Val::nodes).collect();
        let count = columns.first().map_or(0, Vec::len);
        let rows = (0..count)
            .map(|position| {
                columns
                    .iter()
                    .map(|column| {
                        column
                            .get(position)
                            .copied()
                            .ok_or(Internal::missed("array element"))
                    })
                    .collect()
            })
            .collect::<Result<_, _>>()?;
        Ok(Some((shape, rows)))
    }

    fn array(
        &mut self,
        expr: ExprId,
        elements: &[ExprId],
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        if elements.is_empty() {
            return Ok(Some(Val::Error));
        }
        let vals: Vec<Option<Val>> = elements
            .iter()
            .map(|element| self.val(Query::Expr(*element, scope), need))
            .collect();
        if vals.iter().any(Option::is_none) {
            return Ok(None);
        }
        let vals: Vec<Val> = vals.into_iter().flatten().collect();
        if vals.iter().any(|val| matches!(val, Val::Error)) {
            return Ok(Some(Val::Error));
        }
        if vals.iter().any(|val| self.holds_array(val)) {
            let val = self.opaque_array(vals.clone(), loc, scope)?;
            if matches!(val, Val::Array(_)) {
                let members = self.members(elements, &vals, scope)?;
                self.literal_group(expr, members, loc, scope)?;
            }
            return Ok(Some(val));
        }
        let inner = vals.first().map(Val::shape).unwrap_or_default();
        if vals.iter().any(|val| val.shape() != inner) {
            self.error(Code::RaggedArray, loc, scope, RAGGED)?;
            return Ok(Some(Val::Error));
        }
        let members = self.members(elements, &vals, scope)?;
        self.literal_group(expr, members, loc, scope)?;
        let shape = inner.wrap(crate::semantics::analysis::index::index(vals.len()));
        let elements = vals.iter().flat_map(Val::nodes).collect();
        Ok(Some(Val::array(shape, elements)))
    }

    pub(in crate::semantics::analysis) fn held(
        &mut self,
        expr: ExprId,
        val: Option<Val>,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let Some(val) = val else {
            return Ok(None);
        };
        let Some(group) = self.memo.literal_groups.get(&(expr, scope)).copied() else {
            return Ok(Some(val));
        };
        if self.template_of(scope)?.is_none() {
            let Some(sig) = self.sig(group, need) else {
                return Ok(None);
            };
            if sig.is_error() {
                return Ok(Some(Val::Error));
            }
        }
        Ok(Some(self.group_reads(group, val)?))
    }

    pub(in crate::semantics::analysis) fn held_reads(
        &mut self,
        expr: ExprId,
        val: Val,
        scope: ScopeId,
    ) -> Result<Val, Internal> {
        match self.memo.literal_groups.get(&(expr, scope)).copied() {
            Some(group) => self.group_reads(group, val),
            None => Ok(val),
        }
    }

    fn group_reads(&mut self, group: NodeId, val: Val) -> Result<Val, Internal> {
        let Val::Array(array) = &val else {
            return Ok(val);
        };
        if array
            .elements
            .iter()
            .all(|leaf| self.memo.groups.contains_key(leaf))
        {
            self.memo.held_reads.extend(array.elements.iter().copied());
            return Ok(val);
        }
        let mut reads = Vec::with_capacity(array.elements.len());
        for leaf in &array.elements {
            let (loc, scope) = {
                let record = self.node(*leaf)?;
                (record.loc, record.scope)
            };
            reads.push(self.chosen_read(*leaf, group, loc, scope)?);
        }
        self.memo.held_reads.extend(reads.iter().copied());
        Ok(Val::array(array.shape.clone(), reads))
    }

    fn members(
        &self,
        elements: &[ExprId],
        vals: &[Val],
        scope: ScopeId,
    ) -> Result<Vec<NodeId>, Internal> {
        let mut members = Vec::with_capacity(vals.len());
        for (element, val) in elements.iter().zip(vals) {
            match (self.memo.literal_groups.get(&(*element, scope)), val) {
                (Some(group), Val::Array(_)) => members.push(*group),
                (_, Val::Scalar(node)) if self.arrays.contains(node) => members.extend(
                    self.memo
                        .projections
                        .get(node)
                        .ok_or(Internal::missed("projection"))?
                        .iter()
                        .copied(),
                ),
                _ => members.extend(val.nodes()),
            }
        }
        Ok(members)
    }

    fn literal_group(
        &mut self,
        expr: ExprId,
        members: Vec<NodeId>,
        loc: Loc,
        scope: ScopeId,
    ) -> Result<(), Internal> {
        let group = self.push_node(Op::Array(members), loc, scope)?;
        self.memo.literal_groups.insert((expr, scope), group);
        self.memo.group_list.push(group);
        Ok(())
    }

    fn opaque_array(&mut self, vals: Vec<Val>, loc: Loc, scope: ScopeId) -> Result<Val, Internal> {
        let Some(inner) = vals
            .iter()
            .find(|val| !self.holds_array(val))
            .map(Val::shape)
        else {
            let mut nodes = Vec::with_capacity(vals.len());
            for val in &vals {
                nodes.extend(self.boxed(val, loc, scope)?);
            }
            return self.scalar(Op::Array(nodes), loc, scope);
        };
        let mut elements = Vec::new();
        for val in &vals {
            match val {
                Val::Scalar(node) if self.arrays.contains(node) => {
                    let projected =
                        self.projected(*node, &inner, (Code::RaggedArray, RAGGED), loc, scope)?;
                    elements.extend(projected);
                }
                other if other.shape() != inner => {
                    self.error(Code::RaggedArray, loc, scope, RAGGED)?;
                    return Ok(Val::Error);
                }
                other => elements.extend(other.nodes()),
            }
        }
        let shape = inner.wrap(crate::semantics::analysis::index::index(vals.len()));
        Ok(Val::array(shape, elements))
    }

    fn match_val(
        &mut self,
        scrutinee: ExprId,
        arms: &[crate::semantics::ir::MatchArm],
        otherwise: Option<ExprId>,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<Val>, Internal> {
        let scrutinee_val = self.val(Query::Expr(scrutinee, scope), need);
        let mut labels = Vec::new();
        let mut values = Vec::new();
        for arm in arms {
            let label = match &self.hir.expr(arm.label)?.kind {
                ExprKind::Name(_) => Some(Val::Error),
                ExprKind::Error
                | ExprKind::Number(..)
                | ExprKind::Bool(_)
                | ExprKind::Str(_)
                | ExprKind::Variant { .. }
                | ExprKind::To
                | ExprKind::Convert(..)
                | ExprKind::Array(_)
                | ExprKind::Field(..)
                | ExprKind::Index(..)
                | ExprKind::Call(..)
                | ExprKind::Unary(..)
                | ExprKind::Binary(..)
                | ExprKind::If(..)
                | ExprKind::Match { .. } => self.val(Query::Expr(arm.label, scope), need),
            };
            labels.push(label);
            let value = self.val(Query::Expr(arm.value, scope), need);
            values.push(self.held(arm.value, value, scope, need)?);
        }
        let otherwise_val = match otherwise {
            Some(expr) => {
                let value = self.val(Query::Expr(expr, scope), need);
                Some(self.held(expr, value, scope, need)?)
            }
            None => None,
        };
        if scrutinee_val.is_none()
            || labels.iter().any(Option::is_none)
            || values.iter().any(Option::is_none)
            || otherwise_val.as_ref().is_some_and(Option::is_none)
        {
            return Ok(None);
        }
        if let Some(scrutinee_val) = &scrutinee_val {
            self.single_operand(
                scrutinee_val,
                Code::MatchScrutinee,
                "a `match` selects by a single value, not an array",
            );
        }
        let scrutinee_node = match scrutinee_val {
            Some(Val::Scalar(node)) => node,
            Some(Val::Array(_)) => {
                self.error(
                    Code::MatchScrutinee,
                    self.hir.expr(scrutinee)?.loc,
                    scope,
                    "a `match` selects by a single value, not an array",
                )?;
                return Ok(Some(Val::Error));
            }
            Some(Val::Error) | None => return Ok(Some(Val::Error)),
        };
        let mut label_nodes = Vec::new();
        for (arm, label) in arms.iter().zip(labels) {
            let node = match (&self.hir.expr(arm.label)?.kind, label) {
                (ExprKind::Name(name), _) => {
                    let label_loc = self.hir.expr(arm.label)?.loc;
                    self.push_node(Op::Bare(name.clone()), label_loc, scope)?
                }
                (_, Some(Val::Scalar(node))) => node,
                _ => return Ok(Some(Val::Error)),
            };
            label_nodes.push(node);
        }
        let mut all: Vec<Val> = values.into_iter().flatten().collect();
        let has_otherwise = otherwise_val.is_some();
        if let Some(Some(value)) = otherwise_val {
            all.push(value);
        }
        let all = match some!(self.branches(all, scrutinee_node, (loc, scope), need)?) {
            Branches::Whole(mut nodes) => {
                let otherwise = if has_otherwise { nodes.pop() } else { None };
                let arms = label_nodes.iter().copied().zip(nodes).collect();
                return Ok(Some(self.scalar(
                    Op::Match {
                        scrutinee: scrutinee_node,
                        arms,
                        otherwise,
                    },
                    loc,
                    scope,
                )?));
            }
            Branches::Each(vals) => vals,
        };
        let Some((shape, rows)) = self.zip_shapes(&all, loc, scope)? else {
            return Ok(Some(Val::Error));
        };
        let elements = rows
            .into_iter()
            .map(|row| {
                let mut row = row;
                let otherwise = if has_otherwise { row.pop() } else { None };
                let arms = label_nodes.iter().copied().zip(row).collect();
                self.push_node(
                    Op::Match {
                        scrutinee: scrutinee_node,
                        arms,
                        otherwise,
                    },
                    loc,
                    scope,
                )
            })
            .collect::<Result<_, _>>()?;
        Ok(Some(self.shaped(shape, elements)))
    }
}

type Zipped = (Shape, Vec<Vec<NodeId>>);

const OPERAND: &str = "operators and conversions apply to single values, not arrays";

const RAGGED: &str = "the elements of an array are arrays of one length, or all single values";

const BRANCHES: &str = "the branches hold arrays of different shapes";

enum Branches {
    Whole(Vec<NodeId>),
    Each(Vec<Val>),
}
