use symphonyscript_core::{BinaryOp, Scalar, UnaryOp};

use super::hir_builder::HirBuilder;
use super::number_literal::number_literal;
use super::scalar_type::scalar_type;
use crate::diagnostics::Code;
use crate::semantics::ir::{
    Args, ExprId, ExprKind, ForExpr, HistoryForm, Ident, Iterable, LetDecl, Loc, LoopVar, MatchArm,
    SliceBound, StrPart, TypeRef,
};
use crate::syntax::ast::{self, AstNode, StringPart};
use crate::syntax::{SyntaxKind, TokenView};

enum Shape {
    Leaf(ExprKind),
    Paren,
    Unary(UnaryOp),
    Binary(BinaryOp),
    If,
    Convert(Scalar),
    Array(usize),
    Field(Ident),
    Index,
    IndexFromEnd,
    Slice {
        lower: Option<bool>,
        upper: Option<bool>,
    },
    History {
        target: Ident,
        fallback: bool,
        form: HistoryForm,
    },
    Positional(usize),
    Named(Vec<Ident>),
    Str(Vec<Option<String>>),
    For(Box<ForShape>),
}

struct ForShape {
    names: Vec<LoopVar>,
    range: bool,
    accumulator: Option<Ident>,
    accumulator_ty: Option<TypeRef>,
    lets: Vec<(Option<Ident>, Loc, Option<ExprId>)>,
}

enum Frame<'p> {
    Enter(Option<ast::Expr<'p>>, Loc),
    Exit(Shape, Loc, usize),
}

impl HirBuilder<'_> {
    pub(super) fn expr(&mut self, root: Option<ast::Expr<'_>>, fallback: Loc) -> ExprId {
        let mut frames = vec![Frame::Enter(root, fallback)];
        let mut built: Vec<ExprId> = Vec::new();
        while let Some(frame) = frames.pop() {
            crate::semantics::counted_walk::CountedWalk::ExpressionsExpr.step();
            match frame {
                Frame::Enter(None, loc) => built.push(self.push(ExprKind::Error, loc)),
                Frame::Enter(Some(expr), _) => {
                    let loc = self.loc(expr.syntax().range());
                    let (shape, children) = self.shape(&expr);
                    frames.push(Frame::Exit(shape, loc, children.len()));
                    frames.extend(
                        children
                            .into_iter()
                            .rev()
                            .map(|child| Frame::Enter(child, loc)),
                    );
                }
                Frame::Exit(shape, loc, count) => {
                    let start = built.len().saturating_sub(count);
                    let children = built.split_off(start);
                    let id = self.finish(shape, loc, children);
                    built.push(id);
                }
            }
        }
        built
            .pop()
            .unwrap_or_else(|| self.push(ExprKind::Error, fallback))
    }

    pub(super) fn match_block(&mut self, block: &ast::MatchBlock<'_>) -> ExprId {
        let range = block.syntax().range();
        let last_arm = block
            .else_arm()
            .map(|arm| arm.syntax().range().end)
            .or_else(|| block.arms().last().map(|arm| arm.syntax().range().end));
        let loc = self.loc(range.start..last_arm.unwrap_or(range.end));
        let scrutinee = self.expr(block.scrutinee(), loc);
        let arms = block
            .arms()
            .map(|arm| {
                let arm_loc = self.loc(arm.syntax().range());
                MatchArm {
                    label: self.expr(arm.label(), arm_loc),
                    value: self.expr(arm.value(), arm_loc),
                }
            })
            .collect();
        let otherwise = block.else_arm().map(|arm| {
            let arm_loc = self.loc(arm.syntax().range());
            self.expr(arm.value(), arm_loc)
        });
        self.push(
            ExprKind::Match {
                scrutinee,
                arms,
                otherwise,
            },
            loc,
        )
    }

    fn shape<'p>(&mut self, expr: &ast::Expr<'p>) -> (Shape, Vec<Option<ast::Expr<'p>>>) {
        match expr {
            ast::Expr::If(node) => (
                Shape::If,
                vec![node.condition(), node.then_branch(), node.else_branch()],
            ),
            ast::Expr::For(node) => {
                if self.faults.touch(node.syntax().range()) {
                    return (Shape::Leaf(ExprKind::Error), Vec::new());
                }
                self.for_shape(node)
            }
            ast::Expr::Binary(node) => match node
                .op()
                .filter(|op| !self.faults.touch(op.range()))
                .and_then(|op| binary_op(&op, node))
            {
                Some(op) => (Shape::Binary(op), vec![node.lhs(), node.rhs()]),
                None => (Shape::Leaf(ExprKind::Error), Vec::new()),
            },
            ast::Expr::Prefix(node) => {
                let op = node
                    .op()
                    .filter(|op| !self.faults.touch(op.range()))
                    .map(|op| op.kind());
                let operand = node.operand();
                if op == Some(SyntaxKind::Minus) {
                    self.negation(operand)
                } else if op == Some(SyntaxKind::NotKw) {
                    (Shape::Unary(UnaryOp::Not), vec![operand])
                } else {
                    (Shape::Leaf(ExprKind::Error), Vec::new())
                }
            }
            ast::Expr::Paren(node) => (Shape::Paren, vec![node.inner()]),
            ast::Expr::Literal(node) => (Shape::Leaf(self.literal(node)), Vec::new()),
            ast::Expr::String(node) => self.string(node),
            ast::Expr::NameRef(node) => {
                let kind = match node.name() {
                    Some(name) if !self.faults.touch(name.range()) => ExprKind::Name(name.text()),
                    _ => ExprKind::Error,
                };
                (Shape::Leaf(kind), Vec::new())
            }
            ast::Expr::VariantRef(node) => {
                let kind = match (self.ident(node.enum_name()), self.ident(node.variant())) {
                    (Some(enumeration), Some(variant)) => ExprKind::Variant {
                        enumeration,
                        variant,
                    },
                    _ => ExprKind::Error,
                };
                (Shape::Leaf(kind), Vec::new())
            }
            ast::Expr::To(_) => (Shape::Leaf(ExprKind::To), Vec::new()),
            ast::Expr::Conversion(node) => {
                match node.target().and_then(|token| scalar_type(&token)) {
                    Some(target) => (Shape::Convert(target), vec![node.operand()]),
                    None => (Shape::Leaf(ExprKind::Error), Vec::new()),
                }
            }
            ast::Expr::Array(node) => {
                let elements: Vec<_> = node.elements().map(Some).collect();
                (Shape::Array(elements.len()), elements)
            }
            ast::Expr::Field(node) => match self.ident(node.field()) {
                Some(field) if field.text == "stepped" => match node.receiver() {
                    Some(ast::Expr::NameRef(receiver)) => match self.ident(receiver.name()) {
                        Some(target) => (Shape::Leaf(ExprKind::Stepped(target)), Vec::new()),
                        None => (Shape::Leaf(ExprKind::Error), Vec::new()),
                    },
                    receiver => (Shape::Field(field), vec![receiver]),
                },
                Some(field) => (Shape::Field(field), vec![node.receiver()]),
                None => (Shape::Leaf(ExprKind::Error), Vec::new()),
            },
            ast::Expr::Index(node) => {
                if !node.is_slice() {
                    let bound = node.index();
                    let from_end = bound.as_ref().is_some_and(ast::Bound::from_end);
                    let shape = if from_end {
                        Shape::IndexFromEnd
                    } else {
                        Shape::Index
                    };
                    return (
                        shape,
                        vec![node.base(), bound.and_then(|bound| bound.value())],
                    );
                }
                let lower = node.lower();
                let upper = node.upper();
                let mut children = vec![node.base()];
                children.extend(lower.iter().map(ast::Bound::value));
                children.extend(upper.iter().map(ast::Bound::value));
                (
                    Shape::Slice {
                        lower: lower.as_ref().map(ast::Bound::from_end),
                        upper: upper.as_ref().map(ast::Bound::from_end),
                    },
                    children,
                )
            }
            ast::Expr::History(node) => match self.ident(node.name()) {
                Some(target) => {
                    let fallback = node.fallback();
                    let mut children = vec![node.offset()];
                    if fallback.is_some() {
                        children.push(fallback);
                    }
                    (
                        Shape::History {
                            target,
                            fallback: children.len() > 1,
                            form: HistoryForm::Braced,
                        },
                        children,
                    )
                }
                None => (Shape::Leaf(ExprKind::Error), Vec::new()),
            },
            ast::Expr::Call(node) => {
                if let Some(ast::Expr::Field(callee)) = node.callee()
                    && callee.field().is_some_and(|field| field.text() == "past")
                {
                    return self.past_shape(node, &callee);
                }
                let Some(args) = node.args() else {
                    return (Shape::Leaf(ExprKind::Error), Vec::new());
                };
                let positional: Vec<_> = args.positional().map(Some).collect();
                let named: Vec<_> = args.named().collect();
                if !positional.is_empty() && !named.is_empty() {
                    return (Shape::Leaf(ExprKind::Error), Vec::new());
                }
                if named.is_empty() {
                    let mut children = vec![node.callee()];
                    children.extend(positional);
                    return (Shape::Positional(children.len() - 1), children);
                }
                let mut names = Vec::new();
                let mut children = vec![node.callee()];
                for arg in named {
                    let Some(name) = self.ident(arg.name()) else {
                        return (Shape::Leaf(ExprKind::Error), Vec::new());
                    };
                    names.push(name);
                    children.push(arg.value());
                }
                (Shape::Named(names), children)
            }
        }
    }

    fn finish(&mut self, shape: Shape, loc: Loc, children: Vec<ExprId>) -> ExprId {
        let mut operands = children.into_iter();
        let mut next = |builder: &mut Self| {
            operands
                .next()
                .unwrap_or_else(|| builder.push(ExprKind::Error, loc))
        };
        match shape {
            Shape::Leaf(kind) => self.push(kind, loc),
            Shape::Paren => next(self),
            Shape::Unary(op) => {
                let operand = next(self);
                self.push(ExprKind::Unary(op, operand), loc)
            }
            Shape::Binary(op) => {
                let lhs = next(self);
                let rhs = next(self);
                self.push(ExprKind::Binary(op, lhs, rhs), loc)
            }
            Shape::If => {
                let condition = next(self);
                let then = next(self);
                let otherwise = next(self);
                self.push(ExprKind::If(condition, then, otherwise), loc)
            }
            Shape::Convert(target) => {
                let operand = next(self);
                self.push(ExprKind::Convert(target, operand), loc)
            }
            Shape::Array(count) => {
                let elements = (0..count).map(|_| next(self)).collect();
                self.push(ExprKind::Array(elements), loc)
            }
            Shape::Field(field) => {
                let receiver = next(self);
                self.push(ExprKind::Field(receiver, field), loc)
            }
            Shape::Index => {
                let base = next(self);
                let index = next(self);
                self.push(ExprKind::Index(base, index), loc)
            }
            Shape::IndexFromEnd => {
                let base = next(self);
                let index = next(self);
                self.push(ExprKind::IndexFromEnd(base, index), loc)
            }
            Shape::Slice { lower, upper } => {
                let base = next(self);
                let lower = lower.map(|from_end| SliceBound {
                    value: next(self),
                    from_end,
                });
                let upper = upper.map(|from_end| SliceBound {
                    value: next(self),
                    from_end,
                });
                self.push(ExprKind::Slice { base, lower, upper }, loc)
            }
            Shape::History {
                target,
                fallback,
                form,
            } => {
                let offset = next(self);
                let fallback = fallback.then(|| next(self));
                self.push(
                    ExprKind::History {
                        target,
                        offset,
                        fallback,
                        form,
                    },
                    loc,
                )
            }
            Shape::Positional(count) => {
                let callee = next(self);
                let args = (0..count).map(|_| next(self)).collect();
                self.push(ExprKind::Call(callee, Args::Positional(args)), loc)
            }
            Shape::Named(names) => {
                let callee = next(self);
                let args = names.into_iter().map(|name| (name, next(self))).collect();
                self.push(ExprKind::Call(callee, Args::Named(args)), loc)
            }
            Shape::Str(parts) => {
                let parts = parts
                    .into_iter()
                    .map(|part| match part {
                        Some(text) => StrPart::Text(text),
                        None => StrPart::Hole(next(self)),
                    })
                    .collect();
                self.push(ExprKind::Str(parts), loc)
            }
            Shape::For(shape) => {
                let ForShape {
                    names,
                    range,
                    accumulator,
                    accumulator_ty,
                    lets,
                } = *shape;
                let iterable = if range {
                    let start = next(self);
                    Iterable::Range(start, next(self))
                } else {
                    Iterable::Values(next(self))
                };
                let start = next(self);
                let lets = lets
                    .into_iter()
                    .map(|(name, let_loc, built)| LetDecl {
                        name,
                        ty: None,
                        value: built.unwrap_or_else(|| next(self)),
                        loc: let_loc,
                    })
                    .collect();
                let body = next(self);
                self.push(
                    ExprKind::For(Box::new(ForExpr {
                        names,
                        iterable,
                        accumulator,
                        accumulator_ty,
                        start,
                        lets,
                        body,
                    })),
                    loc,
                )
            }
        }
    }

    fn past_shape<'p>(
        &mut self,
        node: &ast::CallExpr<'p>,
        callee: &ast::FieldExpr<'p>,
    ) -> (Shape, Vec<Option<ast::Expr<'p>>>) {
        let error = (Shape::Leaf(ExprKind::Error), Vec::new());
        let target = match callee.receiver() {
            Some(ast::Expr::NameRef(receiver)) => self.ident(receiver.name()),
            Some(receiver) => {
                self.draw(
                    Code::HistoryTarget,
                    receiver.syntax().range(),
                    "only a cell has a history, and `.past` reads the history of the cell it follows by name",
                );
                None
            }
            None => None,
        };
        let (Some(target), Some(args)) = (target, node.args()) else {
            return error;
        };
        let range = node.syntax().range();
        if args.named().next().is_some() {
            self.draw(
                Code::NamedBuiltinArguments,
                range,
                "`past` takes positional arguments only",
            );
            return error;
        }
        let positional: Vec<_> = args.positional().map(Some).collect();
        if !matches!(positional.len(), 1 | 2) {
            self.draw(
                Code::ArgumentCount,
                range,
                &format!(
                    "`past` takes the offset and an optional fallback, 1 or 2 arguments, not {}",
                    positional.len()
                ),
            );
            return error;
        }
        let shape = Shape::History {
            target,
            fallback: positional.len() == 2,
            form: HistoryForm::Past,
        };
        (shape, positional)
    }

    fn for_shape<'p>(&mut self, node: &ast::ForExpr<'p>) -> (Shape, Vec<Option<ast::Expr<'p>>>) {
        let Some(head) = node.head() else {
            return (Shape::Leaf(ExprKind::Error), Vec::new());
        };
        let loop_head = head.loop_head();
        let names = loop_head
            .names()
            .into_iter()
            .map(|loop_name| LoopVar {
                name: self.ident(Some(loop_name.name)),
                ty: loop_name.ty.map(|ty| self.type_ref(&ty)),
            })
            .collect();
        let mut children = Vec::new();
        let range = match loop_head.iterable() {
            Some(ast::Iterable::Range(range)) => {
                children.extend([range.start(), range.end()]);
                true
            }
            Some(ast::Iterable::Expr(expr)) => {
                children.push(Some(expr));
                false
            }
            None => {
                children.push(None);
                false
            }
        };
        children.push(head.start());
        let mut lets = Vec::new();
        for local in node.lets() {
            let let_loc = self.loc(local.syntax().range());
            let name = self.ident(local.name());
            match local.match_block() {
                Some(block) => lets.push((name, let_loc, Some(self.match_block(&block)))),
                None => {
                    lets.push((name, let_loc, None));
                    children.push(local.value());
                }
            }
        }
        children.push(node.body());
        let shape = ForShape {
            names,
            range,
            accumulator: self.ident(head.accumulator()),
            accumulator_ty: head.accumulator_type().map(|ty| self.type_ref(&ty)),
            lets,
        };
        (Shape::For(Box::new(shape)), children)
    }

    fn negation<'p>(&self, operand: Option<ast::Expr<'p>>) -> (Shape, Vec<Option<ast::Expr<'p>>>) {
        let Some(ast::Expr::Literal(literal)) = &operand else {
            return (Shape::Unary(UnaryOp::Neg), vec![operand]);
        };
        match self.literal(literal) {
            ExprKind::Number(number, suffix) => {
                let negated = match number.negated() {
                    Some(negated) => ExprKind::Number(negated, suffix),
                    None => ExprKind::Error,
                };
                (Shape::Leaf(negated), Vec::new())
            }
            ExprKind::Error => (Shape::Paren, vec![operand]),
            ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Name(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Convert(..)
            | ExprKind::Array(_)
            | ExprKind::Field(..)
            | ExprKind::Index(..)
            | ExprKind::IndexFromEnd(..)
            | ExprKind::Slice { .. }
            | ExprKind::History { .. }
            | ExprKind::Stepped(_)
            | ExprKind::Call(..)
            | ExprKind::Unary(..)
            | ExprKind::Binary(..)
            | ExprKind::If(..)
            | ExprKind::Match { .. }
            | ExprKind::For(_) => (Shape::Unary(UnaryOp::Neg), vec![operand]),
        }
    }

    fn literal(&self, node: &ast::Literal<'_>) -> ExprKind {
        let Some(token) = node.token() else {
            return ExprKind::Error;
        };
        if self.faults.touch(token.range()) {
            return ExprKind::Error;
        }
        let kind = token.kind();
        if kind == SyntaxKind::Number {
            return match number_literal(token.text(), self.vocabulary) {
                Some((number, suffix)) => ExprKind::Number(number, suffix),
                None => ExprKind::Error,
            };
        }
        BOOL_LITERALS
            .iter()
            .find(|(literal, _)| *literal == kind)
            .map_or(ExprKind::Error, |(_, value)| ExprKind::Bool(*value))
    }

    fn string<'p>(&self, node: &ast::StringLit<'p>) -> (Shape, Vec<Option<ast::Expr<'p>>>) {
        let error = (Shape::Leaf(ExprKind::Error), Vec::new());
        if !node.is_terminated() {
            return error;
        }
        let mut parts = Vec::new();
        let mut holes = Vec::new();
        for part in node.parts() {
            match part {
                StringPart::Text(token) => {
                    if self.faults.touch(token.range()) {
                        return error;
                    }
                    match unescape(token.text()) {
                        Some(text) => parts.push(Some(text)),
                        None => return error,
                    }
                }
                StringPart::Interpolation(hole) => {
                    parts.push(None);
                    holes.push(hole.value());
                }
            }
        }
        (Shape::Str(parts), holes)
    }
}

const BOOL_LITERALS: [(SyntaxKind, bool); 2] =
    [(SyntaxKind::TrueKw, true), (SyntaxKind::FalseKw, false)];

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

fn binary_op(token: &TokenView<'_>, node: &ast::BinaryExpr<'_>) -> Option<BinaryOp> {
    let kind = token.kind();
    if kind == SyntaxKind::IsKw && node.is_not() {
        return Some(BinaryOp::IsNot);
    }
    BINARY_OPS
        .iter()
        .find(|(token, _)| *token == kind)
        .map(|(_, op)| *op)
}

fn unescape(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        crate::semantics::counted_walk::CountedWalk::ExpressionsUnescape.step();
        if c != '\\' {
            out.push(c);
            continue;
        }
        let escaped = match chars.next()? {
            '"' => '"',
            '\\' => '\\',
            'n' => '\n',
            't' => '\t',
            '{' => '{',
            _ => return None,
        };
        out.push(escaped);
    }
    Some(out)
}
