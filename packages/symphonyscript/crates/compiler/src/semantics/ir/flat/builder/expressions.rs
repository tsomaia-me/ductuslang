use symphonyscript_core::BinaryOp;

use super::super::NONE;
use super::super::float_entry::FloatEntry;
use super::super::name_kind::NameKind;
use super::super::op::Op;
use super::super::part::Part;
use super::super::{binary_op_code, index};
use super::constructs::{DOCUMENT_LITERAL, LAMBDA, LOCAL_ANNOTATION};
use super::ir_builder::{IrBuilder, span};
use super::marks::Marks;
use super::types::scalar_code;
use super::value::{Local, Pending, Value};
use crate::semantics::ir::{Number, Suffix, number_literal, scalar_type};
use crate::syntax::ast::{self, AstNode, StringPart};
use crate::syntax::{NodeView, SyntaxKind, TokenView};

impl<'p> IrBuilder<'p, '_> {
    pub(super) fn expression(
        &mut self,
        view: NodeView<'p>,
        base: usize,
        marks: Marks,
    ) -> Option<Value> {
        let whole = span(&view);
        let node = match view.kind() {
            SyntaxKind::Literal => self.literal(view, marks),
            SyntaxKind::StringLit => self.string(view, base, marks),
            SyntaxKind::NameRef => {
                let name = ast::NameRef::cast(view)?.name();
                let name = self.name_of(name, NameKind::Reference);
                if self.named(name) {
                    self.emit(Op::Name, (name, 0, 0), whole)
                } else {
                    self.bad(whole)
                }
            }
            SyntaxKind::VariantRef => {
                let node = ast::VariantRef::cast(view)?;
                let enumeration = self.name_of(node.enum_name(), NameKind::TypeName);
                let variant = self.name_of(node.variant(), NameKind::Variant);
                if self.named(enumeration) && self.named(variant) {
                    self.emit(Op::Variant, (enumeration, variant, 0), whole)
                } else {
                    self.bad(whole)
                }
            }
            SyntaxKind::ToExpr => self.emit(Op::To, (0, 0, 0), whole),
            SyntaxKind::Conversion => {
                let node = ast::Conversion::cast(view)?;
                match node.target().and_then(|token| scalar_type(&token)) {
                    Some(target) => {
                        let operand = self.expr_of(base, node.operand(), whole);
                        self.emit(Op::Convert, (operand, scalar_code(target), 0), whole)
                    }
                    None => self.dead(marks, whole),
                }
            }
            SyntaxKind::ArrayLit => {
                let children: Vec<NodeView<'p>> = view.children().collect();
                let mut elements = Vec::with_capacity(children.len());
                for child in children {
                    let element = match ast::Expr::cast(child) {
                        Some(expr) => self.expr_of(base, Some(expr), whole),
                        None => self.bad(whole),
                    };
                    elements.push(element);
                }
                let cut = self.first_in(whole.0..whole.1).is_some();
                let start = index(self.ir.kids.len());
                let count = index(elements.len());
                self.ir.kids.extend(elements);
                self.emit(Op::Array, (start, count, u32::from(cut)), whole)
            }
            SyntaxKind::FieldExpr => return self.field(view, base, marks),
            SyntaxKind::IndexExpr => self.index_expr(view, base),
            SyntaxKind::Bound => {
                let value = ast::Bound::cast(view)?.value();
                let value = value.and_then(|value| self.value_at(base, value.syntax()));
                return Some(match self.force(value) {
                    Some(id) => Value::Expr(id),
                    None => Value::Nothing,
                });
            }
            SyntaxKind::Interpolation => {
                let value = ast::Interpolation::cast(view)?.value();
                let value = value.and_then(|value| self.value_at(base, value.syntax()));
                return Some(match self.force(value) {
                    Some(id) => Value::Expr(id),
                    None => Value::Nothing,
                });
            }
            SyntaxKind::ParenExpr => {
                let inner = ast::ParenExpr::cast(view)?.inner();
                self.expr_of(base, inner, whole)
            }
            SyntaxKind::PrefixExpr => self.prefix(view, base, marks),
            SyntaxKind::BinaryExpr => self.binary(view, base, marks),
            SyntaxKind::IfExpr => {
                let node = ast::IfExpr::cast(view)?;
                let condition = self.expr_of(base, node.condition(), whole);
                let then = self.expr_of(base, node.then_branch(), whole);
                let otherwise = self.expr_of(base, node.else_branch(), whole);
                self.emit(Op::If, (condition, then, otherwise), whole)
            }
            SyntaxKind::CallExpr => self.call(view, base, marks),
            SyntaxKind::ForExpr | SyntaxKind::ForExprBlock => self.for_expr(view, base, marks),
            SyntaxKind::ForExprHead => return Some(self.for_head(view, base)),
            SyntaxKind::RangeExpr => {
                let node = ast::RangeExpr::cast(view)?;
                let start = node
                    .start()
                    .and_then(|start| self.value_at(base, start.syntax()));
                let end = node.end().and_then(|end| self.value_at(base, end.syntax()));
                let start = self.force(start).unwrap_or(NONE);
                let end = self.force(end).unwrap_or(NONE);
                return Some(Value::Range { start, end });
            }
            SyntaxKind::MatchBlock => self.match_block(view, base, marks),
            SyntaxKind::MatchArm => return self.match_arm(view, base),
            SyntaxKind::ElseArm => {
                let value = ast::ElseArm::cast(view)?.value();
                return Some(Value::Else(self.expr_of(base, value, whole)));
            }
            SyntaxKind::JsonLit => self.unbuilt_expr(marks, whole, DOCUMENT_LITERAL),
            SyntaxKind::RecordLit => {
                self.unbuilt_expr(marks, whole, "a record literal (SPEC §3.8)")
            }
            SyntaxKind::MapLit => self.unbuilt_expr(marks, whole, "a map literal (SPEC §3.10)"),
            _ => return None,
        };
        Some(Value::Expr(node))
    }

    pub(super) fn dead(&mut self, marks: Marks, whole: (u32, u32)) -> u32 {
        marks.truncate(self.ir);
        self.bad(whole)
    }

    fn unbuilt_expr(&mut self, marks: Marks, whole: (u32, u32), construct: &str) -> u32 {
        self.unbuilt_at(marks, whole, whole, construct)
    }

    fn unbuilt_at(
        &mut self,
        marks: Marks,
        at: (u32, u32),
        whole: (u32, u32),
        construct: &str,
    ) -> u32 {
        marks.truncate(self.ir);
        let from = self.drawn_mark();
        self.draw_unbuilt(at, construct);
        self.bad_drawn(whole, from)
    }

    fn literal(&mut self, view: NodeView<'_>, marks: Marks) -> u32 {
        let whole = span(&view);
        let Some(token) = ast::Literal::cast(view).and_then(|node| node.token()) else {
            return self.bad(whole);
        };
        if token.kind() == SyntaxKind::NoneKw {
            return self.unbuilt_expr(marks, whole, "the value `none` (SPEC §3.9)");
        }
        if self.touch(token.range()) {
            return self.bad(whole);
        }
        match token.kind() {
            SyntaxKind::Number => match number_literal(token.text(), self.vocabulary) {
                Some((number, suffix)) => self.number(number, suffix, whole),
                None => self.bad(whole),
            },
            SyntaxKind::TrueKw => self.emit(Op::Bool, (1, 0, 0), whole),
            SyntaxKind::FalseKw => self.emit(Op::Bool, (0, 0, 0), whole),
            _ => self.bad(whole),
        }
    }

    fn number(&mut self, number: Number, suffix: Suffix, whole: (u32, u32)) -> u32 {
        let suffix = match suffix {
            Suffix::None => NONE,
            suffix => {
                self.ir.suffixes.push(suffix);
                index(self.ir.suffixes.len() - 1)
            }
        };
        match number {
            Number::Int(value) => {
                self.ir.ints.push(value);
                let at = index(self.ir.ints.len() - 1);
                self.emit(Op::Int, (at, suffix, 0), whole)
            }
            Number::Float(float) => {
                let value = float.to_f64().unwrap_or(f64::NAN);
                let start = index(self.ir.texts.len());
                self.ir.texts.push_str(&float.digits);
                self.ir.floats.push(FloatEntry {
                    negative: float.negative,
                    digits: (start, index(self.ir.texts.len())),
                    exponent: float.exponent,
                    value,
                });
                let at = index(self.ir.floats.len() - 1);
                self.emit(Op::Float, (at, suffix, 0), whole)
            }
        }
    }

    fn string(&mut self, view: NodeView<'p>, base: usize, marks: Marks) -> u32 {
        let whole = span(&view);
        let Some(node) = ast::StringLit::cast(view) else {
            return self.dead(marks, whole);
        };
        if !node.is_terminated() {
            return self.dead(marks, whole);
        }
        let mut parts = Vec::new();
        for part in node.parts() {
            match part {
                StringPart::Text(token) => {
                    if self.touch(token.range()) {
                        return self.dead(marks, whole);
                    }
                    let start = index(self.ir.texts.len());
                    if !unescape_into(token.text(), &mut self.ir.texts) {
                        return self.dead(marks, whole);
                    }
                    parts.push(Part::Text(start, index(self.ir.texts.len())));
                }
                StringPart::Interpolation(hole) => {
                    let value = self.value_at(base, hole.syntax());
                    let id = match value {
                        Some(Value::Expr(id)) => id,
                        _ => self.bad(whole),
                    };
                    parts.push(Part::Hole(id));
                }
            }
        }
        let start = index(self.ir.parts.len());
        let count = index(parts.len());
        self.ir.parts.extend(parts);
        self.emit(Op::Str, (start, count, 0), whole)
    }

    fn field(&mut self, view: NodeView<'p>, base: usize, marks: Marks) -> Option<Value> {
        let whole = span(&view);
        let node = ast::FieldExpr::cast(view)?;
        let field = self.name_of(node.field(), NameKind::PathSegment);
        let receiver_view = node.receiver();
        let receiver = receiver_view
            .as_ref()
            .and_then(|receiver| self.value_at(base, receiver.syntax()));
        let receiver = self.force(receiver).unwrap_or(NONE);
        let special = self
            .ir
            .name_text(field)
            .is_some_and(|text| text == "past" || text == "stepped");
        if special {
            let receiver_name = if let Ok((Op::Name, name, _, _)) = self.ir.node(receiver) {
                name
            } else {
                NONE
            };
            let receiver_range = receiver_view
                .as_ref()
                .map_or(whole, |receiver| span(receiver.syntax()));
            return Some(Value::Pending(Pending {
                receiver,
                receiver_name,
                receiver_range,
                receiver_is_name_ref: matches!(receiver_view, Some(ast::Expr::NameRef(_))),
                receiver_missing: receiver_view.is_none(),
                field,
                lo: whole.0,
                hi: whole.1,
            }));
        }
        if !self.named(field) {
            return Some(Value::Expr(self.dead(marks, whole)));
        }
        let (receiver, missing) = if receiver == NONE {
            (self.bad(whole), 1)
        } else {
            (receiver, 0)
        };
        Some(Value::Expr(self.emit(
            Op::Field,
            (receiver, field, missing),
            whole,
        )))
    }

    pub(super) fn force_pending(&mut self, pending: Pending) -> u32 {
        let whole = (pending.lo, pending.hi);
        if !self.named(pending.field) {
            return self.bad(whole);
        }
        let stepped = self.ir.name_text(pending.field) == Some("stepped");
        if stepped && pending.receiver_is_name_ref {
            if pending.receiver_name == NONE {
                return self.bad(whole);
            }
            return self.emit(Op::Stepped, (pending.receiver, pending.field, 0), whole);
        }
        let (receiver, missing) = if pending.receiver_missing {
            (self.bad(whole), 1)
        } else {
            (pending.receiver, 0)
        };
        self.emit(Op::Field, (receiver, pending.field, missing), whole)
    }

    fn index_expr(&mut self, view: NodeView<'p>, base: usize) -> u32 {
        let whole = span(&view);
        let Some(node) = ast::IndexExpr::cast(view) else {
            return self.bad(whole);
        };
        let target = self.expr_of(base, node.base(), whole);
        let bound_value = |builder: &mut Self, bound: &ast::Bound<'_>| -> u32 {
            match builder.value_at(base, bound.syntax()) {
                Some(Value::Expr(id)) => id,
                _ => builder.bad(whole),
            }
        };
        if !node.is_slice() {
            let bound = node.index();
            let from_end = bound.as_ref().is_some_and(ast::Bound::from_end);
            let at = match &bound {
                Some(bound) => bound_value(self, bound),
                None => self.bad(whole),
            };
            let op = if from_end {
                Op::IndexFromEnd
            } else {
                Op::Index
            };
            return self.emit(op, (target, at, 0), whole);
        }
        let lower = node.lower();
        let upper = node.upper();
        let lower_value = match &lower {
            Some(bound) => bound_value(self, bound),
            None => NONE,
        };
        let upper_value = match &upper {
            Some(bound) => bound_value(self, bound),
            None => NONE,
        };
        let flags = u32::from(lower.as_ref().is_some_and(ast::Bound::from_end))
            | (u32::from(upper.as_ref().is_some_and(ast::Bound::from_end)) << 1);
        let start = index(self.ir.kids.len());
        self.ir.kids.extend([lower_value, upper_value]);
        self.emit(Op::Slice, (target, start, flags), whole)
    }

    fn prefix(&mut self, view: NodeView<'p>, base: usize, marks: Marks) -> u32 {
        let whole = span(&view);
        let Some(node) = ast::PrefixExpr::cast(view) else {
            return self.bad(whole);
        };
        let op = node
            .op()
            .filter(|op| !self.touch(op.range()))
            .map(|op| op.kind());
        let operand = node.operand();
        match op {
            Some(SyntaxKind::Minus) => {}
            Some(SyntaxKind::NotKw) => {
                let operand = self.expr_of(base, operand, whole);
                return self.emit(Op::Not, (operand, 0, 0), whole);
            }
            _ => return self.dead(marks, whole),
        }
        let is_literal = matches!(operand, Some(ast::Expr::Literal(_)));
        let operand = self.expr_of(base, operand, whole);
        if is_literal {
            let node = self.ir.node(operand);
            if let Ok((Op::Int, at, _, _)) = node {
                let negated = self
                    .ir
                    .ints
                    .get(at as usize)
                    .and_then(|value| value.checked_neg());
                match (negated, self.ir.ints.get_mut(at as usize)) {
                    (Some(negated), Some(slot)) => *slot = negated,
                    _ => self.set_node(operand, Op::Bad, (0, 0, NONE)),
                }
                self.set_span(operand, whole);
                return operand;
            }
            if let Ok((Op::Float, at, _, _)) = node {
                if let Some(float) = self.ir.floats.get_mut(at as usize) {
                    float.negative = !float.negative;
                    float.value = -float.value;
                }
                self.set_span(operand, whole);
                return operand;
            }
            if let Ok((Op::Bad, ..)) = node {
                return operand;
            }
        }
        self.emit(Op::Neg, (operand, 0, 0), whole)
    }

    fn binary(&mut self, view: NodeView<'p>, base: usize, marks: Marks) -> u32 {
        let whole = span(&view);
        let Some(node) = ast::BinaryExpr::cast(view) else {
            return self.bad(whole);
        };
        if let Some(alt) = node.alt_type() {
            return self.unbuilt_at(
                marks,
                span(alt.syntax()),
                whole,
                "a type after `is` (SPEC §5.3)",
            );
        }
        let op = node.op();
        match op.as_ref().map(TokenView::kind) {
            Some(SyntaxKind::Question2) => {
                return self.unbuilt_expr(marks, whole, "the fallback `??` (SPEC §5.3)");
            }
            Some(SyntaxKind::WithoutKw) => {
                return self.unbuilt_expr(marks, whole, "the operator `without` (SPEC §3.10)");
            }
            Some(_) | None => {}
        }
        let binary = op
            .filter(|op| !self.touch(op.range()))
            .and_then(|op| binary_kind(&op, &node));
        let Some(binary) = binary else {
            return self.dead(marks, whole);
        };
        let lhs = self.expr_of(base, node.lhs(), whole);
        let rhs = self.expr_of(base, node.rhs(), whole);
        self.emit(binary_op_code(binary), (lhs, rhs, 0), whole)
    }

    fn match_arm(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let whole = span(&view);
        let node = ast::MatchArm::cast(view)?;
        let (unbuilt, at) = if let Some(alt) = node.alt_type() {
            (0, span(alt.syntax()))
        } else if let Some(ast::Expr::String(label)) = node.label() {
            (1, span(label.syntax()))
        } else {
            (NONE, whole)
        };
        let label = self.expr_of(base, node.label(), whole);
        let value = self.expr_of(base, node.value(), whole);
        Some(Value::Arm {
            label,
            value,
            unbuilt,
            lo: at.0,
            hi: at.1,
        })
    }

    fn match_block(&mut self, view: NodeView<'p>, base: usize, marks: Marks) -> u32 {
        let whole = span(&view);
        let Some(block) = ast::MatchBlock::cast(view) else {
            return self.bad(whole);
        };
        let arm_end = |value: Option<ast::Expr<'_>>, arm: &NodeView<'_>| {
            value.map_or(arm.range().end, |value| value.syntax().range().end)
        };
        let last_arm = block
            .else_arm()
            .map(|arm| arm_end(arm.value(), arm.syntax()))
            .or_else(|| {
                block
                    .arms()
                    .last()
                    .map(|arm| arm_end(arm.value(), arm.syntax()))
            });
        let loc = (whole.0, last_arm.unwrap_or(whole.1));
        let arms: Vec<Value> = block
            .arms()
            .filter_map(|arm| self.value_at(base, arm.syntax()))
            .collect();
        let unbuilt: Vec<(u32, (u32, u32))> = arms
            .iter()
            .filter_map(|arm| match arm {
                Value::Arm {
                    unbuilt, lo, hi, ..
                } if *unbuilt != NONE => Some((*unbuilt, (*lo, *hi))),
                _ => None,
            })
            .collect();
        if !unbuilt.is_empty() {
            marks.truncate(self.ir);
            let from = self.drawn_mark();
            for (kind, at) in unbuilt {
                let construct = if kind == 0 {
                    "a type as a `match` label (SPEC §5.4)"
                } else {
                    "a string `match` label (SPEC §5.4)"
                };
                self.draw_unbuilt(at, construct);
            }
            return self.bad_drawn(loc, from);
        }
        let scrutinee = self.expr_of(base, block.scrutinee(), loc);
        let mut kids = Vec::with_capacity(arms.len() * 2 + 1);
        for arm in arms {
            if let Value::Arm { label, value, .. } = arm {
                kids.extend([label, value]);
            }
        }
        if let Some(Value::Else(value)) = block
            .else_arm()
            .and_then(|arm| self.value_at(base, arm.syntax()))
        {
            kids.push(value);
        }
        let start = index(self.ir.kids.len());
        let count = index(kids.len());
        self.ir.kids.extend(kids);
        self.emit(Op::Match, (scrutinee, start, count), loc)
    }

    pub(super) fn local_value(&mut self, view: NodeView<'p>, base: usize) -> Option<Value> {
        let node = ast::LocalLet::cast(view)?;
        let name = self.name_of(node.name(), NameKind::Binder);
        let ty = node.ty().and_then(|ty| self.value_at(base, ty.syntax()));
        let ty = match ty {
            Some(Value::Ty { root, .. }) => root,
            _ => NONE,
        };
        let (value, is_match) = match node.match_block() {
            Some(block) => (self.force(self.value_at(base, block.syntax())), true),
            None => {
                let value = node
                    .value()
                    .and_then(|value| self.value_at(base, value.syntax()));
                (self.force(value), false)
            }
        };
        let whole = span(&view);
        Some(Value::Local(Local {
            name,
            ty,
            value: value.unwrap_or(NONE),
            is_match,
            lo: whole.0,
            hi: whole.1,
        }))
    }

    fn for_expr(&mut self, view: NodeView<'p>, base: usize, marks: Marks) -> u32 {
        let whole = span(&view);
        let Some(node) = ast::ForExpr::cast(view) else {
            return self.bad(whole);
        };
        if self.touch(whole.0..whole.1) {
            return self.dead(marks, whole);
        }
        if let Some(lambda) = node.lets().find_map(|local| local.lambda()) {
            return self.unbuilt_at(marks, span(lambda.syntax()), whole, LAMBDA);
        }
        if let Some(ty) = node.lets().find_map(|local| local.ty()) {
            return self.unbuilt_at(marks, span(ty.syntax()), whole, LOCAL_ANNOTATION);
        }
        let head = node
            .head()
            .and_then(|head| self.value_at(base, head.syntax()));
        let Some(Value::Head(record)) = head else {
            return self.dead(marks, whole);
        };
        let Some(mut rec) = self.ir.fors.get(record as usize).copied() else {
            return self.dead(marks, whole);
        };
        rec.iterable = if rec.range {
            (
                self.or_bad(rec.iterable.0, whole),
                self.or_bad(rec.iterable.1, whole),
            )
        } else {
            (self.or_bad(rec.iterable.0, whole), NONE)
        };
        rec.start = self.or_bad(rec.start, whole);
        let lets_start = index(self.ir.lets.len());
        for local in node.lets() {
            if let Some(Value::Local(local)) = self.value_at(base, local.syntax()) {
                self.local_let(local, record, whole);
            }
        }
        rec.lets = (lets_start, index(self.ir.lets.len()));
        rec.body = self.expr_of(base, node.body(), whole);
        if let Some(slot) = self.ir.fors.get_mut(record as usize) {
            *slot = rec;
        }
        self.emit(Op::For, (record, 0, 0), whole)
    }

    fn for_head(&mut self, view: NodeView<'p>, base: usize) -> Value {
        let Some(head) = ast::ForExprHead::cast(view) else {
            return Value::Nothing;
        };
        let loop_head = head.loop_head();
        let names = self.loop_names(
            &loop_head,
            base,
            super::super::binder_kind::BinderKind::ForExprLoop,
        );
        let (range, iterable) = self.iterable_of(&loop_head, base);
        let accumulator = self.name_of(head.accumulator(), NameKind::Binder);
        let accumulator_ty = head
            .accumulator_type()
            .and_then(|ty| self.value_at(base, ty.syntax()));
        let accumulator_ty = self.commit_type(accumulator_ty);
        let accumulator = self.binder(
            accumulator,
            super::super::binder_kind::BinderKind::Accumulator,
            accumulator_ty,
            span(&view),
        );
        let start = head
            .start()
            .and_then(|start| self.value_at(base, start.syntax()));
        let start = self.force(start).unwrap_or(NONE);
        self.ir.fors.push(super::super::for_rec::ForRec {
            names,
            range,
            iterable,
            accumulator,
            start,
            lets: (0, 0),
            body: NONE,
        });
        Value::Head(index(self.ir.fors.len() - 1))
    }
}

fn binary_kind(token: &TokenView<'_>, node: &ast::BinaryExpr<'_>) -> Option<BinaryOp> {
    let kind = token.kind();
    if kind == SyntaxKind::IsKw && node.is_not() {
        return Some(BinaryOp::IsNot);
    }
    BINARY_OPS
        .iter()
        .find(|(token, _)| *token == kind)
        .map(|(_, op)| *op)
}

const BINARY_OPS: [(SyntaxKind, BinaryOp); 14] = [
    (SyntaxKind::Plus, BinaryOp::Add),
    (SyntaxKind::Minus, BinaryOp::Sub),
    (SyntaxKind::Star, BinaryOp::Mul),
    (SyntaxKind::Slash, BinaryOp::Div),
    (SyntaxKind::DivKw, BinaryOp::IntDiv),
    (SyntaxKind::Percent, BinaryOp::Rem),
    (SyntaxKind::Star2, BinaryOp::Pow),
    (SyntaxKind::IsKw, BinaryOp::Is),
    (SyntaxKind::Lt, BinaryOp::Lt),
    (SyntaxKind::Le, BinaryOp::Le),
    (SyntaxKind::Gt, BinaryOp::Gt),
    (SyntaxKind::Ge, BinaryOp::Ge),
    (SyntaxKind::AndKw, BinaryOp::And),
    (SyntaxKind::OrKw, BinaryOp::Or),
];

fn unescape_into(text: &str, out: &mut String) -> bool {
    let start = out.len();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let escaped = match chars.next() {
            Some('"') => '"',
            Some('\\') => '\\',
            Some('n') => '\n',
            Some('t') => '\t',
            Some('{') => '{',
            _ => {
                out.truncate(start);
                return false;
            }
        };
        out.push(escaped);
    }
    true
}
