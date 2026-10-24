use crate::diagnostics::Code;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::ids::{NodeId, ScopeId};
use crate::semantics::analysis::node::{Op, Piece};
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::shape::Shape;
use crate::semantics::analysis::val::Val;
use crate::semantics::ir::{ExprId, ExprKind, Loc, StrPart};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn run_expr(
        &mut self,
        expr: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> bool {
        let hir = self.hir;
        let node = hir.expr(expr);
        let loc = node.loc;
        let val = match &node.kind {
            ExprKind::Error | ExprKind::To => Some(Val::Error),
            ExprKind::Number(number, suffix) => {
                Some(self.scalar(Op::Number(number.clone(), *suffix), loc, scope))
            }
            ExprKind::Bool(value) => Some(self.scalar(Op::Bool(*value), loc, scope)),
            ExprKind::Str(parts) => self.string(parts, loc, scope, need),
            ExprKind::Name(name) => self.lookup(name, loc, scope, need),
            ExprKind::Variant {
                enumeration,
                variant,
            } => Some(self.variant(enumeration, variant, loc, scope)),
            ExprKind::Convert(target, operand) => {
                let operand = self.val(Query::Expr(*operand, scope), need);
                operand.map(|operand| {
                    self.unary_val(operand, loc, scope, |operand| Op::Convert(*target, operand))
                })
            }
            ExprKind::Array(elements) => self.array(elements, loc, scope, need),
            ExprKind::Field(base, name) => self.field(*base, name, loc, scope, need),
            ExprKind::Index(base, index) => self.index(*base, *index, loc, scope, need),
            ExprKind::Call(callee, args) => self.call(*callee, args, expr, loc, scope, need),
            ExprKind::Unary(op, operand) => {
                let operand = self.val(Query::Expr(*operand, scope), need);
                operand.map(|operand| {
                    self.unary_val(operand, loc, scope, |operand| Op::Unary(*op, operand))
                })
            }
            ExprKind::Binary(op, lhs, rhs) => {
                let lhs = self.val(Query::Expr(*lhs, scope), need);
                let rhs = self.val(Query::Expr(*rhs, scope), need);
                match (lhs, rhs) {
                    (Some(lhs), Some(rhs)) => Some(self.binary_val(*op, lhs, rhs, loc, scope)),
                    _ => None,
                }
            }
            ExprKind::If(condition, then, otherwise) => {
                let condition = self.val(Query::Expr(*condition, scope), need);
                let then = self.val(Query::Expr(*then, scope), need);
                let otherwise = self.val(Query::Expr(*otherwise, scope), need);
                match (condition, then, otherwise) {
                    (Some(condition), Some(then), Some(otherwise)) => {
                        Some(self.if_val(condition, then, otherwise, loc, scope))
                    }
                    _ => None,
                }
            }
            ExprKind::Match {
                scrutinee,
                arms,
                otherwise,
            } => self.match_val(*scrutinee, arms, *otherwise, loc, scope, need),
        };
        let Some(val) = val else {
            return false;
        };
        self.memo.exprs.insert((expr, scope), val);
        true
    }

    pub(in crate::semantics::analysis) fn scalar(
        &mut self,
        op: Op,
        loc: Loc,
        scope: ScopeId,
    ) -> Val {
        Val::Scalar(self.push_node(op, loc, scope))
    }

    fn string(
        &mut self,
        parts: &[StrPart],
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        let mut values = Vec::new();
        for part in parts {
            if let StrPart::Hole(expr) = part {
                values.push((*expr, self.val(Query::Expr(*expr, scope), need)));
            }
        }
        if values.iter().any(|(_, value)| value.is_none()) {
            return None;
        }
        if values.is_empty() {
            let text = parts
                .iter()
                .map(|part| match part {
                    StrPart::Text(text) => text.as_str(),
                    StrPart::Hole(_) => "",
                })
                .collect::<String>();
            return Some(self.scalar(Op::Str(text), loc, scope));
        }
        let mut holes = values.into_iter();
        let mut pieces = Vec::new();
        for part in parts {
            match part {
                StrPart::Text(text) => pieces.push(Piece::Text(text.clone())),
                StrPart::Hole(_) => match holes.next() {
                    Some((_, Some(Val::Scalar(node)))) => pieces.push(Piece::Value(node)),
                    Some((hole, Some(Val::Array(_)))) => {
                        let hole_loc = self.hir.expr(hole).loc;
                        self.error(
                            Code::InterpolationType,
                            hole_loc,
                            scope,
                            "an interpolation holds a single value, not an array",
                        );
                        return Some(Val::Error);
                    }
                    _ => return Some(Val::Error),
                },
            }
        }
        Some(self.scalar(Op::Concat(pieces), loc, scope))
    }

    fn variant(
        &mut self,
        enumeration: &crate::semantics::ir::Ident,
        variant: &crate::semantics::ir::Ident,
        loc: Loc,
        scope: ScopeId,
    ) -> Val {
        let Some(id) = self.vocabulary.enum_named(&enumeration.text) else {
            self.error(
                Code::UnknownVariant,
                enumeration.loc,
                scope,
                format!("`{}` is not an enum of the vocabulary", enumeration.text),
            );
            return Val::Error;
        };
        let position = self
            .vocabulary
            .enum_decl(id)
            .and_then(|decl| decl.variants.iter().position(|name| *name == variant.text));
        match position.and_then(|position| u16::try_from(position).ok()) {
            Some(index) => self.scalar(Op::Variant(id, index), loc, scope),
            None => {
                self.error(
                    Code::UnknownVariant,
                    variant.loc,
                    scope,
                    format!(
                        "the enum `{}` has no variant `{}`",
                        enumeration.text, variant.text
                    ),
                );
                Val::Error
            }
        }
    }

    pub(in crate::semantics::analysis) fn unary_val(
        &mut self,
        operand: Val,
        loc: Loc,
        scope: ScopeId,
        op: impl Fn(NodeId) -> Op,
    ) -> Val {
        match operand {
            Val::Scalar(operand) => self.scalar(op(operand), loc, scope),
            Val::Array(_) => {
                self.error(
                    Code::ArrayOperand,
                    loc,
                    scope,
                    "operators and conversions apply to single values, not arrays",
                );
                Val::Error
            }
            Val::Error => Val::Error,
        }
    }

    fn binary_val(
        &mut self,
        op: symphonyscript_core::BinaryOp,
        lhs: Val,
        rhs: Val,
        loc: Loc,
        scope: ScopeId,
    ) -> Val {
        match (lhs, rhs) {
            (Val::Scalar(lhs), Val::Scalar(rhs)) => {
                self.scalar(Op::Binary(op, lhs, rhs), loc, scope)
            }
            (Val::Error, _) | (_, Val::Error) => Val::Error,
            _ => {
                self.error(
                    Code::ArrayOperand,
                    loc,
                    scope,
                    "operators apply to single values, not arrays",
                );
                Val::Error
            }
        }
    }

    fn if_val(
        &mut self,
        condition: Val,
        then: Val,
        otherwise: Val,
        loc: Loc,
        scope: ScopeId,
    ) -> Val {
        let condition = match condition {
            Val::Scalar(condition) => condition,
            Val::Array(_) => {
                self.error(
                    Code::ConditionType,
                    loc,
                    scope,
                    "the condition of an `if` is a `bool`, not an array",
                );
                return Val::Error;
            }
            Val::Error => return Val::Error,
        };
        let Some((shape, pairs)) = self.zip_shapes(&[then, otherwise], loc, scope) else {
            return Val::Error;
        };
        let elements: Vec<NodeId> = pairs
            .into_iter()
            .map(|pair| {
                let then = pair.first().copied().unwrap_or(condition);
                let otherwise = pair.get(1).copied().unwrap_or(condition);
                self.push_node(Op::If(condition, then, otherwise), loc, scope)
            })
            .collect();
        self.shaped(shape, elements)
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
    ) -> Option<(Shape, Vec<Vec<NodeId>>)> {
        if vals.iter().any(|val| matches!(val, Val::Error)) {
            return None;
        }
        let shape = vals.first().map(Val::shape).unwrap_or_default();
        if vals.iter().any(|val| val.shape() != shape) {
            self.error(
                Code::TypeMismatch,
                loc,
                scope,
                "the branches hold arrays of different shapes",
            );
            return None;
        }
        let columns: Vec<Vec<NodeId>> = vals.iter().map(Val::nodes).collect();
        let count = columns.first().map_or(0, Vec::len);
        let rows = (0..count)
            .map(|position| {
                columns
                    .iter()
                    .filter_map(|column| column.get(position).copied())
                    .collect()
            })
            .collect();
        Some((shape, rows))
    }

    fn array(
        &mut self,
        elements: &[ExprId],
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        let vals: Vec<Option<Val>> = elements
            .iter()
            .map(|element| self.val(Query::Expr(*element, scope), need))
            .collect();
        if vals.iter().any(Option::is_none) {
            return None;
        }
        let vals: Vec<Val> = vals.into_iter().flatten().collect();
        if vals.iter().any(|val| matches!(val, Val::Error)) {
            return Some(Val::Error);
        }
        let inner = vals.first().map(Val::shape).unwrap_or_default();
        if vals.iter().any(|val| val.shape() != inner) {
            self.error(
                Code::RaggedArray,
                loc,
                scope,
                "the elements of an array are arrays of one length, or all single values",
            );
            return Some(Val::Error);
        }
        let shape = inner.wrap(crate::semantics::analysis::engine::index(vals.len()));
        let elements = vals.iter().flat_map(Val::nodes).collect();
        Some(Val::array(shape, elements))
    }

    fn match_val(
        &mut self,
        scrutinee: ExprId,
        arms: &[crate::semantics::ir::MatchArm],
        otherwise: Option<ExprId>,
        loc: Loc,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Option<Val> {
        let scrutinee_val = self.val(Query::Expr(scrutinee, scope), need);
        let mut labels = Vec::new();
        let mut values = Vec::new();
        for arm in arms {
            let label = match &self.hir.expr(arm.label).kind {
                ExprKind::Name(_) => Some(Val::Error),
                _ => self.val(Query::Expr(arm.label, scope), need),
            };
            labels.push(label);
            values.push(self.val(Query::Expr(arm.value, scope), need));
        }
        let otherwise_val = otherwise.map(|expr| self.val(Query::Expr(expr, scope), need));
        if scrutinee_val.is_none()
            || labels.iter().any(Option::is_none)
            || values.iter().any(Option::is_none)
            || otherwise_val.as_ref().is_some_and(Option::is_none)
        {
            return None;
        }
        let scrutinee_node = match scrutinee_val {
            Some(Val::Scalar(node)) => node,
            Some(Val::Array(_)) => {
                self.error(
                    Code::MatchScrutinee,
                    self.hir.expr(scrutinee).loc,
                    scope,
                    "a `match` selects by a single value, not an array",
                );
                return Some(Val::Error);
            }
            _ => return Some(Val::Error),
        };
        let mut label_nodes = Vec::new();
        for (arm, label) in arms.iter().zip(labels) {
            let node = match (&self.hir.expr(arm.label).kind, label) {
                (ExprKind::Name(name), _) => {
                    let label_loc = self.hir.expr(arm.label).loc;
                    self.push_node(Op::Bare(name.clone()), label_loc, scope)
                }
                (_, Some(Val::Scalar(node))) => node,
                _ => return Some(Val::Error),
            };
            label_nodes.push(node);
        }
        let mut all: Vec<Val> = values.into_iter().flatten().collect();
        let has_otherwise = otherwise_val.is_some();
        if let Some(Some(value)) = otherwise_val {
            all.push(value);
        }
        let Some((shape, rows)) = self.zip_shapes(&all, loc, scope) else {
            return Some(Val::Error);
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
            .collect();
        Some(self.shaped(shape, elements))
    }
}
