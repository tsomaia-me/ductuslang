use crate::semantics::Internal;
use crate::semantics::analysis::engine::Engine;
use crate::semantics::analysis::fast_hash::FastMap;
use crate::semantics::analysis::ids::ScopeId;
use crate::semantics::analysis::is_float::is_float;
use crate::semantics::analysis::kind::Kind;
use crate::semantics::analysis::locals_owner::LocalsOwner;
use crate::semantics::analysis::param_source::ParamSource;
use crate::semantics::analysis::query::Query;
use crate::semantics::analysis::resolve::Found;
use crate::semantics::analysis::scope_kind::ScopeKind;
use crate::semantics::analysis::some::some;
use crate::semantics::analysis::unify_kinds::unify_kinds;
use crate::semantics::analysis::val::Val;
use crate::semantics::collection::Decl;
use crate::semantics::ir::{ExprId, ExprKind};
use symphonyscript_core::{BinaryOp, UnaryOp};

impl Engine<'_> {
    pub(in crate::semantics::analysis) fn arm_type(
        &mut self,
        root: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<ArmType>, Internal> {
        let mut found: FastMap<(ExprId, ScopeId), Option<ArmType>> = FastMap::default();
        let mut parts: FastMap<(ExprId, ScopeId), Vec<(ExprId, ScopeId)>> = FastMap::default();
        let mut stack = vec![(root, scope, false)];
        while let Some((expr, scope, ready)) = stack.pop() {
            crate::semantics::counted_walk::CountedWalk::CallsArmType.step();
            if found.contains_key(&(expr, scope)) {
                continue;
            }
            if !ready {
                let children = self.arm_parts(expr, scope)?;
                stack.push((expr, scope, true));
                stack.extend(children.iter().map(|(expr, scope)| (*expr, *scope, false)));
                parts.insert((expr, scope), children);
                continue;
            }
            let children = parts.remove(&(expr, scope)).unwrap_or_default();
            let recursion = self.calls_recursive(expr, scope)?;
            let tainted = recursion
                || children
                    .iter()
                    .any(|child| found.get(child).is_some_and(Option::is_some));
            let typed = if tainted {
                let mut types = Vec::with_capacity(children.len());
                for child in &children {
                    let known = match found.get(child).cloned().flatten() {
                        Some(known) => known,
                        None => some!(self.clean_arm_type(child.0, child.1, need)?),
                    };
                    types.push(known);
                }
                Some(if recursion {
                    ArmType::UNKNOWN
                } else {
                    self.combined_arm_type(expr, &types)?
                })
            } else {
                None
            };
            found.insert((expr, scope), typed);
        }
        match found.remove(&(root, scope)).flatten() {
            Some(typed) => Ok(Some(typed)),
            None => self.clean_arm_type(root, scope, need),
        }
    }

    fn arm_parts(
        &mut self,
        expr: ExprId,
        scope: ScopeId,
    ) -> Result<Vec<(ExprId, ScopeId)>, Internal> {
        let kind = self.hir.expr(expr)?.kind.clone();
        Ok(match kind {
            ExprKind::Name(name) => match self.find(&name, scope)? {
                Found::Query(Query::Let(owner, local)) => {
                    let value = self
                        .let_decl(owner, local)?
                        .map(|decl| decl.value)
                        .ok_or(Internal::missed("local let"))?;
                    vec![(value, self.let_scope(owner, local))]
                }
                Found::Query(Query::Param(owner, index)) => {
                    match self.param_default(owner, index)? {
                        Some(default) => vec![(default, self.prefix_scope(owner, index)?)],
                        None => Vec::new(),
                    }
                }
                Found::Val(_)
                | Found::Query(_)
                | Found::Item(_)
                | Found::Decl(_)
                | Found::Placeholder
                | Found::Nothing => Vec::new(),
            },
            ExprKind::Call(_, args) => args
                .values()
                .into_iter()
                .map(|value| (value, scope))
                .collect(),
            ExprKind::Match {
                arms, otherwise, ..
            } => arms
                .iter()
                .map(|arm| arm.value)
                .chain(otherwise)
                .map(|value| (value, scope))
                .collect(),
            other @ (ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Convert(..)
            | ExprKind::Array(_)
            | ExprKind::Field(..)
            | ExprKind::Index(..)
            | ExprKind::IndexFromEnd(..)
            | ExprKind::Slice { .. }
            | ExprKind::History { .. }
            | ExprKind::Unary(..)
            | ExprKind::Binary(..)
            | ExprKind::If(..)) => other
                .children()
                .into_iter()
                .map(|child| (child, scope))
                .collect(),
        })
    }

    fn param_default(&self, scope: ScopeId, index: u32) -> Result<Option<ExprId>, Internal> {
        let ScopeKind::Locals(locals) = &self.scope_of(scope)?.kind else {
            return Ok(None);
        };
        let (LocalsOwner::Fn(function), Some(ParamSource::Default)) =
            (locals.owner, locals.params.get(index as usize))
        else {
            return Ok(None);
        };
        Ok(self
            .hir
            .fn_decl(function)?
            .params
            .get(index as usize)
            .and_then(|param| param.default))
    }

    fn calls_recursive(&mut self, expr: ExprId, scope: ScopeId) -> Result<bool, Internal> {
        let ExprKind::Call(callee, _) = &self.hir.expr(expr)?.kind else {
            return Ok(false);
        };
        let ExprKind::Name(name) = &self.hir.expr(*callee)?.kind else {
            return Ok(false);
        };
        let name = name.clone();
        Ok(match self.find(&name, scope)? {
            Found::Decl(Decl::Fn(function)) => self.is_recursive(function)?,
            Found::Val(_)
            | Found::Query(_)
            | Found::Item(_)
            | Found::Decl(_)
            | Found::Placeholder
            | Found::Nothing => false,
        })
    }

    fn clean_arm_type(
        &mut self,
        expr: ExprId,
        scope: ScopeId,
        need: &mut Vec<Query>,
    ) -> Result<Option<ArmType>, Internal> {
        let val = some!(self.val(Query::Expr(expr, scope), need));
        let known_kind = |kind: Kind| match kind {
            Kind::Int | Kind::Float | Kind::Bool | Kind::Enum(_) | Kind::Str | Kind::Node => {
                Some(kind)
            }
            Kind::Bare | Kind::Any | Kind::Error => None,
        };
        Ok(Some(match val {
            Val::Error => ArmType::UNKNOWN,
            Val::Array(array) => {
                let mut kind = None;
                for element in &array.elements {
                    let element_kind = known_kind(some!(self.sig(*element, need)).kind);
                    kind = unified(kind, element_kind);
                }
                let lengths = array.shape.lengths();
                let mut inner: Vec<u32> = lengths.iter().skip(1).copied().collect();
                inner.reverse();
                ArmType {
                    kind,
                    shape: ArmShape::Array((lengths != [0]).then_some(inner)),
                }
            }
            Val::Scalar(node) => {
                let kind = known_kind(some!(self.sig(node, need)).kind);
                let shape = if self.arrays.contains(&node) {
                    match some!(self.layout_of(node, need)?).shaped {
                        Ok(None) => ArmShape::Single,
                        Ok(Some(layout)) => {
                            let empty = layout.outer() == Some(0) && layout.inner().is_empty();
                            ArmShape::Array((!empty).then(|| layout.inner().to_vec()))
                        }
                        Err(_) => ArmShape::Unknown,
                    }
                } else {
                    ArmShape::Single
                };
                ArmType { kind, shape }
            }
        }))
    }

    fn combined_arm_type(&self, expr: ExprId, types: &[ArmType]) -> Result<ArmType, Internal> {
        let all_kinds = || {
            types
                .iter()
                .fold(None, |kind, typed| unified(kind, typed.kind))
        };
        Ok(match &self.hir.expr(expr)?.kind {
            ExprKind::If(..) => {
                let arms = types.get(1..).unwrap_or_default();
                ArmType {
                    kind: arms
                        .iter()
                        .fold(None, |kind, typed| unified(kind, typed.kind)),
                    shape: arms
                        .iter()
                        .fold(ArmShape::Unknown, |shape, typed| shape.meet(&typed.shape)),
                }
            }
            ExprKind::Match { .. } => ArmType {
                kind: all_kinds(),
                shape: types
                    .iter()
                    .fold(ArmShape::Unknown, |shape, typed| shape.meet(&typed.shape)),
            },
            ExprKind::Binary(op, ..) => match op {
                BinaryOp::Add => {
                    let array = types.iter().find_map(|typed| match &typed.shape {
                        ArmShape::Array(inner) => Some(inner.clone()),
                        ArmShape::Unknown | ArmShape::Single => None,
                    });
                    let single = types.iter().any(|typed| typed.shape == ArmShape::Single);
                    ArmType {
                        kind: all_kinds(),
                        shape: match array {
                            Some(inner) => ArmShape::Array(inner),
                            None if single => ArmShape::Single,
                            None => ArmShape::Unknown,
                        },
                    }
                }
                BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::IntDiv
                | BinaryOp::Rem
                | BinaryOp::Pow => ArmType {
                    kind: all_kinds(),
                    shape: ArmShape::Single,
                },
                BinaryOp::Div => ArmType {
                    kind: Some(Kind::Float),
                    shape: ArmShape::Single,
                },
                BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge
                | BinaryOp::Is
                | BinaryOp::IsNot
                | BinaryOp::And
                | BinaryOp::Or => ArmType {
                    kind: Some(Kind::Bool),
                    shape: ArmShape::Single,
                },
            },
            ExprKind::Unary(UnaryOp::Neg, _) => ArmType {
                kind: all_kinds(),
                shape: ArmShape::Single,
            },
            ExprKind::Unary(UnaryOp::Not, _) => ArmType {
                kind: Some(Kind::Bool),
                shape: ArmShape::Single,
            },
            ExprKind::Convert(target, _) => ArmType {
                kind: Some(if is_float(*target) {
                    Kind::Float
                } else {
                    Kind::Int
                }),
                shape: ArmShape::Single,
            },
            ExprKind::Array(_) => ArmType {
                kind: all_kinds(),
                shape: ArmShape::Array(
                    types
                        .iter()
                        .all(|typed| typed.shape == ArmShape::Single)
                        .then(Vec::new),
                ),
            },
            ExprKind::Index(..) | ExprKind::IndexFromEnd(..) => {
                let base = types.first().cloned().unwrap_or(ArmType::UNKNOWN);
                ArmType {
                    kind: base.kind,
                    shape: match base.shape {
                        ArmShape::Array(Some(mut inner)) => match inner.pop() {
                            Some(_) => ArmShape::Array(Some(inner)),
                            None => ArmShape::Single,
                        },
                        ArmShape::Array(None) | ArmShape::Unknown | ArmShape::Single => {
                            ArmShape::Unknown
                        }
                    },
                }
            }
            ExprKind::Slice { .. } | ExprKind::Name(_) => {
                types.first().cloned().unwrap_or(ArmType::UNKNOWN)
            }
            ExprKind::Error
            | ExprKind::Number(..)
            | ExprKind::Bool(_)
            | ExprKind::Str(_)
            | ExprKind::Variant { .. }
            | ExprKind::To
            | ExprKind::Field(..)
            | ExprKind::History { .. }
            | ExprKind::Call(..) => ArmType::UNKNOWN,
        })
    }
}

#[derive(Clone, PartialEq)]
pub(crate) enum ArmShape {
    Unknown,
    Single,
    Array(Option<Vec<u32>>),
}

impl ArmShape {
    fn meet(&self, other: &ArmShape) -> ArmShape {
        match (self, other) {
            (ArmShape::Unknown, known) | (known, ArmShape::Unknown) => known.clone(),
            (ArmShape::Array(Some(a)), ArmShape::Array(Some(b))) if a != b => ArmShape::Unknown,
            (ArmShape::Array(a), ArmShape::Array(b)) => ArmShape::Array(a.clone().or(b.clone())),
            (ArmShape::Single, ArmShape::Single) => ArmShape::Single,
            (ArmShape::Single, ArmShape::Array(_)) | (ArmShape::Array(_), ArmShape::Single) => {
                ArmShape::Unknown
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct ArmType {
    pub kind: Option<Kind>,
    pub shape: ArmShape,
}

impl ArmType {
    const UNKNOWN: ArmType = ArmType {
        kind: None,
        shape: ArmShape::Unknown,
    };
}

fn unified(current: Option<Kind>, other: Option<Kind>) -> Option<Kind> {
    match (current, other) {
        (Some(current), Some(other)) => unify_kinds(current, other),
        (None, other) => other,
        (current, None) => current,
    }
}
