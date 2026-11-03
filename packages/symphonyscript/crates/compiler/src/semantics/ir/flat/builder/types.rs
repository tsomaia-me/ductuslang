use symphonyscript_core::Scalar;

use super::super::NONE;
use super::super::name_kind::NameKind;
use super::super::op::Op;
use super::constructs::{
    FN_TYPE, INNER_CAPACITY, JSON_TYPE, MAP_TYPE, NONE_TYPE, OPTIONAL_TYPE, PAREN_TYPE,
    QUALIFIED_NAME, SIZE, STR_TYPE, SUM_TYPE, TYPE_ARGS,
};
use super::ir_builder::{IrBuilder, span};
use super::marks::Marks;
use super::value::Value;
use crate::semantics::ir::scalar_type;
use crate::syntax::ast::{self, AstNode};
use crate::syntax::{ElementView, NodeView, SyntaxKind};

pub(crate) const SCALARS: [Scalar; 6] = [
    Scalar::U8,
    Scalar::I32,
    Scalar::I64,
    Scalar::F32,
    Scalar::F64,
    Scalar::Bool,
];

pub(in crate::semantics::ir::flat) fn scalar_code(scalar: Scalar) -> u32 {
    SCALARS
        .iter()
        .position(|known| *known == scalar)
        .map_or(NONE, |at| u32::try_from(at).unwrap_or(NONE))
}

impl IrBuilder<'_, '_> {
    pub(super) fn type_value(&mut self, view: NodeView<'_>, marks: Marks) -> Value {
        marks.truncate(self.ir);
        let whole = span(&view);
        let pending = |builder: &mut Self, construct: u32, at: (u32, u32)| {
            let from = builder.drawn_mark();
            let root = builder.emit(Op::Bad, (from, from, construct), at);
            Value::Ty { root, base: root }
        };
        let empty_here = view.kind() == SyntaxKind::TypeRef && view.range().is_empty();
        if marks.empty_type || empty_here {
            return pending(self, NONE, whole);
        }
        match view.kind() {
            SyntaxKind::SumType => return pending(self, SUM_TYPE, whole),
            SyntaxKind::FnType => return pending(self, FN_TYPE, whole),
            _ => {}
        }
        let Some(node) = ast::TypeRef::cast(view) else {
            return pending(self, NONE, whole);
        };
        let touched = self.touch(whole.0..whole.1);
        let open = unclosed(&view);
        if !touched
            && !open
            && let Some((at, construct)) = unbuilt_part(&view)
        {
            return pending(self, construct, at);
        }
        let base = match node.base() {
            _ if touched || open => self.bad(whole),
            Some(token) => match scalar_type(&token) {
                Some(scalar) => self.emit(Op::TyScalar, (scalar_code(scalar), 0, 0), whole),
                None => {
                    let name = self.name_of(node.enum_name(), NameKind::TypeName);
                    if self.named(name) {
                        self.emit(Op::TyName, (name, 0, 0), whole)
                    } else {
                        self.bad(whole)
                    }
                }
            },
            None => self.bad(whole),
        };
        let capacity = node.capacity();
        let mut root = base;
        for element in view.children_with_tokens() {
            let Some(token) = element.into_token() else {
                continue;
            };
            if token.kind() != SyntaxKind::Number {
                continue;
            }
            let value = super::super::length_value(token.text());
            let operands = (root, value.unwrap_or(0), u32::from(value.is_some()));
            let op = if Some(&token) == capacity.as_ref() {
                Op::TyVar
            } else {
                Op::TyArray
            };
            let range = token.range();
            root = self.emit(op, operands, (range.start, range.end));
        }
        Value::Ty { root, base }
    }
}

fn unclosed(view: &NodeView<'_>) -> bool {
    let depth = view
        .children_with_tokens()
        .filter_map(ElementView::into_token)
        .fold(0i32, |depth, token| match token.kind() {
            SyntaxKind::LBracket => depth + 1,
            SyntaxKind::RBracket => depth - 1,
            _ => depth,
        });
    depth != 0
}

fn unbuilt_part(view: &NodeView<'_>) -> Option<((u32, u32), u32)> {
    let whole = span(view);
    let mut brackets = 0usize;
    let mut capacity_at = None;
    for element in view.children_with_tokens() {
        let found = match &element {
            ElementView::Node(child) => match child.kind() {
                SyntaxKind::StrType => Some(STR_TYPE),
                SyntaxKind::MapType => Some(MAP_TYPE),
                SyntaxKind::JsonType => Some(JSON_TYPE),
                SyntaxKind::ParenType => Some(PAREN_TYPE),
                SyntaxKind::QualifiedName => Some(QUALIFIED_NAME),
                SyntaxKind::TypeArgs => Some(TYPE_ARGS),
                SyntaxKind::Size => Some(SIZE),
                _ => None,
            },
            ElementView::Token(token) => match token.kind() {
                SyntaxKind::NoneKw => Some(NONE_TYPE),
                SyntaxKind::LBracket => {
                    brackets += 1;
                    None
                }
                SyntaxKind::Dot2 => {
                    capacity_at = capacity_at.or(Some(brackets));
                    None
                }
                _ => None,
            },
        };
        if let Some(construct) = found {
            let range = element.range();
            return Some(((range.start, range.end), construct));
        }
    }
    if capacity_at.is_some_and(|at| at < brackets) {
        return Some((whole, INNER_CAPACITY));
    }
    if view
        .children_with_tokens()
        .any(|element| element.kind() == SyntaxKind::Question)
    {
        return Some((whole, OPTIONAL_TYPE));
    }
    None
}
