use symphonyscript_core::UnaryOp;

use super::super::builder::SCALARS;
use super::super::op::Op;
use super::super::part::Part;
use super::super::{NONE, binary_of, index};
use super::hir_adapter::Adapter;
use crate::semantics::Internal;
use crate::semantics::ir::{
    Args, Expr, ExprId, ExprKind, FloatLit, ForExpr, Ident, Iterable, LetDecl, LoopVar, MatchArm,
    Number, SliceBound, StrPart, Suffix, TypeBase, TypeRef,
};

enum Frame {
    Enter(u32),
    Exit(u32, usize),
    Accumulator(u32),
}

impl Adapter<'_> {
    pub fn expr(&mut self, root: u32) -> Result<ExprId, Internal> {
        let mut frames = vec![Frame::Enter(root)];
        let mut built: Vec<ExprId> = Vec::new();
        let mut types: Vec<Option<TypeRef>> = Vec::new();
        let mut loop_vars: Vec<Vec<LoopVar>> = Vec::new();
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(node) => {
                    let children = self.enter(node, &mut loop_vars)?;
                    let count = children
                        .iter()
                        .filter(|child| matches!(child, Frame::Enter(_)))
                        .count();
                    frames.push(Frame::Exit(node, count));
                    for child in children.into_iter().rev() {
                        frames.push(child);
                    }
                }
                Frame::Accumulator(binder) => {
                    let annotation = self.ir.binder(binder)?.annotation;
                    let ty = self.annotation(annotation)?;
                    types.push(ty);
                }
                Frame::Exit(node, count) => {
                    let start = built.len().saturating_sub(count);
                    let children = built.split_off(start);
                    let id = self.exit(node, children, &mut types, &mut loop_vars)?;
                    built.push(id);
                }
            }
        }
        built.pop().ok_or(Internal::missed("adapted expression"))
    }

    fn kids(&self, start: u32, count: u32) -> Result<Vec<u32>, Internal> {
        (start..start + count).map(|at| self.ir.kid(at)).collect()
    }

    fn enter(
        &mut self,
        node: u32,
        loop_vars: &mut Vec<Vec<LoopVar>>,
    ) -> Result<Vec<Frame>, Internal> {
        let (op, a, b, c) = self.ir.node(node)?;
        let enter = |nodes: Vec<u32>| nodes.into_iter().map(Frame::Enter).collect();
        Ok(match op {
            Op::Int | Op::Float | Op::Bool | Op::Name | Op::Variant | Op::To | Op::Stepped => {
                Vec::new()
            }
            Op::Bad => {
                self.emit_drawn((a, b))?;
                Vec::new()
            }
            Op::Str => {
                let mut holes = Vec::new();
                for at in a..a + b {
                    if let Some(Part::Hole(hole)) = self.ir.parts.get(at as usize) {
                        holes.push(*hole);
                    }
                }
                enter(holes)
            }
            Op::Convert | Op::Field | Op::Neg | Op::Not => enter(vec![a]),
            Op::Array => enter(self.kids(a, b)?),
            Op::Index | Op::IndexFromEnd => enter(vec![a, b]),
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
            | Op::IsNot => enter(vec![a, b]),
            Op::Slice => {
                let mut nodes = vec![a];
                nodes.extend(self.kids(b, 2)?.into_iter().filter(|bound| *bound != NONE));
                enter(nodes)
            }
            Op::Call => {
                let mut nodes = vec![a];
                nodes.extend(self.kids(b, c / 2)?);
                enter(nodes)
            }
            Op::NamedCall => {
                let mut nodes = vec![a];
                let pairs = self.kids(b, 2 * (c / 2))?;
                nodes.extend(pairs.chunks(2).filter_map(|pair| pair.get(1).copied()));
                enter(nodes)
            }
            Op::MethodCall => {
                let mut nodes = self.kids(b, c / 2 + 1)?;
                nodes.push(a);
                enter(nodes)
            }
            Op::NamedMethodCall => {
                let all = self.kids(b, 2 * (c / 2) + 1)?;
                let mut nodes: Vec<u32> = all.first().copied().into_iter().collect();
                nodes.extend(
                    all.get(1..)
                        .unwrap_or_default()
                        .chunks(2)
                        .filter_map(|pair| pair.get(1).copied()),
                );
                nodes.push(a);
                enter(nodes)
            }
            Op::Past => {
                let mut nodes = vec![b];
                if c != NONE {
                    nodes.push(c);
                }
                enter(nodes)
            }
            Op::If => enter(vec![a, b, c]),
            Op::Match => {
                let mut nodes = vec![a];
                nodes.extend(self.kids(b, c)?);
                enter(nodes)
            }
            Op::For => {
                let rec = *self
                    .ir
                    .fors
                    .get(a as usize)
                    .ok_or(Internal::missed("IR for"))?;
                let mut vars = Vec::new();
                for at in rec.names.0..rec.names.1 {
                    let binder = *self.ir.binder(at)?;
                    let ty = self.annotation(binder.annotation)?;
                    vars.push(LoopVar {
                        name: self.ident(binder.name),
                        ty,
                    });
                }
                loop_vars.push(vars);
                let mut frames = Vec::new();
                let mut later = Vec::new();
                for at in rec.lets.0..rec.lets.1 {
                    let local = *self
                        .ir
                        .lets
                        .get(at as usize)
                        .ok_or(Internal::missed("IR let"))?;
                    if local.is_match {
                        frames.push(Frame::Enter(local.value));
                    } else {
                        later.push(Frame::Enter(local.value));
                    }
                }
                frames.push(Frame::Accumulator(rec.accumulator));
                frames.push(Frame::Enter(rec.iterable.0));
                if rec.range {
                    frames.push(Frame::Enter(rec.iterable.1));
                }
                frames.push(Frame::Enter(rec.start));
                frames.extend(later);
                frames.push(Frame::Enter(rec.body));
                frames
            }
            Op::TyScalar | Op::TyName | Op::TyArray | Op::TyVar => {
                return Err(Internal::missed("an expression where the IR holds a type"));
            }
        })
    }

    fn ident_of_name_node(&self, node: u32) -> Result<Ident, Internal> {
        let (Op::Name, name, _, _) = self.ir.node(node)? else {
            return Err(Internal::missed("IR name"));
        };
        self.ident(name).ok_or(Internal::missed("IR name"))
    }

    fn suffix(&self, at: u32) -> Suffix {
        if at == NONE {
            return Suffix::None;
        }
        self.ir
            .suffixes
            .get(at as usize)
            .copied()
            .unwrap_or(Suffix::None)
    }

    fn exit(
        &mut self,
        node: u32,
        children: Vec<ExprId>,
        types: &mut Vec<Option<TypeRef>>,
        loop_vars: &mut Vec<Vec<LoopVar>>,
    ) -> Result<ExprId, Internal> {
        let (op, a, b, c) = self.ir.node(node)?;
        let (lo, hi) = self.ir.span(node)?;
        let loc = self.loc(lo, hi);
        let missed = || Internal::missed("adapted operand");
        let mut operands = children.into_iter();
        let mut next = || operands.next().ok_or_else(missed);
        let kind = match op {
            Op::Int => {
                let value = *self.ir.ints.get(a as usize).ok_or_else(missed)?;
                ExprKind::Number(Number::Int(value), self.suffix(b))
            }
            Op::Float => {
                let float = *self.ir.floats.get(a as usize).ok_or_else(missed)?;
                ExprKind::Number(
                    Number::Float(FloatLit {
                        negative: float.negative,
                        digits: self.ir.text(float.digits).to_string(),
                        exponent: float.exponent,
                    }),
                    self.suffix(b),
                )
            }
            Op::Bool => ExprKind::Bool(a == 1),
            Op::Str => {
                let mut parts = Vec::new();
                for at in a..a + b {
                    match self.ir.parts.get(at as usize) {
                        Some(Part::Text(start, end)) => {
                            parts.push(StrPart::Text(self.ir.text((*start, *end)).to_string()));
                        }
                        Some(Part::Hole(_)) => parts.push(StrPart::Hole(next()?)),
                        None => return Err(missed()),
                    }
                }
                ExprKind::Str(parts)
            }
            Op::Name => ExprKind::Name(self.ident(a).ok_or_else(missed)?.text),
            Op::Variant => ExprKind::Variant {
                enumeration: self.ident(a).ok_or_else(missed)?,
                variant: self.ident(b).ok_or_else(missed)?,
            },
            Op::To => ExprKind::To,
            Op::Convert => {
                let scalar = *SCALARS.get(b as usize).ok_or_else(missed)?;
                ExprKind::Convert(scalar, next()?)
            }
            Op::Array => ExprKind::Array((0..b).map(|_| next()).collect::<Result<_, _>>()?),
            Op::Field => {
                let receiver = next()?;
                ExprKind::Field(receiver, self.ident(b).ok_or_else(missed)?)
            }
            Op::Index => {
                let base = next()?;
                ExprKind::Index(base, next()?)
            }
            Op::IndexFromEnd => {
                let base = next()?;
                ExprKind::IndexFromEnd(base, next()?)
            }
            Op::Slice => {
                let base = next()?;
                let bounds = self.kids(b, 2)?;
                let mut bound = |at: usize, flag: u32| -> Result<Option<SliceBound>, Internal> {
                    if bounds.get(at).copied().unwrap_or(NONE) == NONE {
                        return Ok(None);
                    }
                    Ok(Some(SliceBound {
                        value: next()?,
                        from_end: c & flag != 0,
                    }))
                };
                let lower = bound(0, 1)?;
                let upper = bound(1, 2)?;
                ExprKind::Slice { base, lower, upper }
            }
            Op::Call => {
                let callee = next()?;
                let args = (0..c / 2).map(|_| next()).collect::<Result<_, _>>()?;
                ExprKind::Call(callee, Args::Positional(args))
            }
            Op::NamedCall => {
                let callee = next()?;
                let names = self.kids(b, 2 * (c / 2))?;
                let mut args = Vec::new();
                for pair in names.chunks(2) {
                    let name = pair.first().copied().ok_or_else(missed)?;
                    args.push((self.ident(name).ok_or_else(missed)?, next()?));
                }
                ExprKind::Call(callee, Args::Named(args))
            }
            Op::MethodCall => {
                let receiver = next()?;
                let args = (0..c / 2).map(|_| next()).collect::<Result<_, _>>()?;
                let callee = next()?;
                ExprKind::Call(
                    callee,
                    Args::Received(receiver, Box::new(Args::Positional(args))),
                )
            }
            Op::NamedMethodCall => {
                let receiver = next()?;
                let names = self.kids(b + 1, 2 * (c / 2))?;
                let mut args = Vec::new();
                for pair in names.chunks(2) {
                    let name = pair.first().copied().ok_or_else(missed)?;
                    args.push((self.ident(name).ok_or_else(missed)?, next()?));
                }
                let callee = next()?;
                ExprKind::Call(
                    callee,
                    Args::Received(receiver, Box::new(Args::Named(args))),
                )
            }
            Op::Past => {
                let target = self.ident_of_name_node(a)?;
                let offset = next()?;
                let fallback = if c == NONE { None } else { Some(next()?) };
                ExprKind::History {
                    target,
                    offset,
                    fallback,
                }
            }
            Op::Stepped => ExprKind::Stepped(self.ident_of_name_node(a)?),
            Op::Neg => ExprKind::Unary(UnaryOp::Neg, next()?),
            Op::Not => ExprKind::Unary(UnaryOp::Not, next()?),
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
            | Op::IsNot => {
                let binary = binary_of(op).ok_or_else(missed)?;
                let lhs = next()?;
                ExprKind::Binary(binary, lhs, next()?)
            }
            Op::If => {
                let condition = next()?;
                let then = next()?;
                ExprKind::If(condition, then, next()?)
            }
            Op::Match => {
                let scrutinee = next()?;
                let mut arms = Vec::new();
                for _ in 0..c / 2 {
                    let label = next()?;
                    arms.push(MatchArm {
                        label,
                        value: next()?,
                    });
                }
                let otherwise = if c % 2 == 1 { Some(next()?) } else { None };
                ExprKind::Match {
                    scrutinee,
                    arms,
                    otherwise,
                }
            }
            Op::For => {
                let rec = *self.ir.fors.get(a as usize).ok_or_else(missed)?;
                let names = loop_vars.pop().ok_or_else(missed)?;
                let accumulator_ty = types.pop().ok_or_else(missed)?;
                let mut matched = Vec::new();
                let mut plain = Vec::new();
                for at in rec.lets.0..rec.lets.1 {
                    let local = *self.ir.lets.get(at as usize).ok_or_else(missed)?;
                    if local.is_match {
                        matched.push(at);
                    } else {
                        plain.push(at);
                    }
                }
                let mut values = std::collections::BTreeMap::new();
                for at in &matched {
                    values.insert(*at, next()?);
                }
                let iterable = if rec.range {
                    let start = next()?;
                    Iterable::Range(start, next()?)
                } else {
                    Iterable::Values(next()?)
                };
                let start = next()?;
                for at in &plain {
                    values.insert(*at, next()?);
                }
                let body = next()?;
                let mut lets = Vec::new();
                for at in rec.lets.0..rec.lets.1 {
                    let local = *self.ir.lets.get(at as usize).ok_or_else(missed)?;
                    let binder = *self.ir.binder(local.binder)?;
                    lets.push(LetDecl {
                        name: self.ident(binder.name),
                        ty: None,
                        value: *values.get(&at).ok_or_else(missed)?,
                        loc: self.loc(local.lo, local.hi),
                    });
                }
                let accumulator = self.ir.binder(rec.accumulator)?.name;
                ExprKind::For(Box::new(ForExpr {
                    names,
                    iterable,
                    accumulator: self.ident(accumulator),
                    accumulator_ty,
                    start,
                    lets,
                    body,
                }))
            }
            Op::Bad => ExprKind::Error,
            Op::TyScalar | Op::TyName | Op::TyArray | Op::TyVar => return Err(missed()),
        };
        let id = ExprId(index(self.hir.exprs.len()));
        self.hir.exprs.push(Expr { kind, loc });
        Ok(id)
    }

    pub fn type_ref(&mut self, root: u32) -> Result<TypeRef, Internal> {
        let mut lengths = Vec::new();
        let mut capacity = None;
        let mut node = root;
        loop {
            let (op, a, b, c) = self.ir.node(node)?;
            let (lo, hi) = self.ir.span(node)?;
            if op == Op::TyArray {
                lengths.push(((c == 1).then_some(b), self.loc(lo, hi)));
            } else if op == Op::TyVar {
                capacity = Some(((c == 1).then_some(b), self.loc(lo, hi)));
            } else if matches!(op, Op::TyScalar | Op::TyName | Op::Bad) {
                break;
            } else {
                return Err(Internal::missed("IR type"));
            }
            node = a;
        }
        lengths.reverse();
        let (op, a, b, _) = self.ir.node(node)?;
        let (lo, hi) = self.ir.span(node)?;
        let base = if op == Op::TyScalar {
            TypeBase::Scalar(*SCALARS.get(a as usize).ok_or(Internal::missed("IR type"))?)
        } else if op == Op::TyName {
            self.ident(a).map_or(TypeBase::Error, TypeBase::Enum)
        } else {
            self.emit_drawn((a, b))?;
            TypeBase::Error
        };
        Ok(TypeRef {
            base,
            lengths,
            capacity,
            loc: self.loc(lo, hi),
        })
    }
}
