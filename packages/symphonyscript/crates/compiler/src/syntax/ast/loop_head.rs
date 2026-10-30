use std::ops::Range;

use super::ast_node::AstNode;
use super::iterable::Iterable;
use super::loop_name::LoopName;
use super::name::Name;
use super::support::child;
use super::type_ref::TypeRef;
use crate::syntax::{ElementView, NodeView, SyntaxKind};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LoopHead<'p>(NodeView<'p>);

impl<'p> LoopHead<'p> {
    pub(super) fn of(node: &NodeView<'p>) -> Self {
        Self(node.clone())
    }

    pub fn names(&self) -> Vec<LoopName<'p>> {
        let mut names: Vec<LoopName<'p>> = Vec::new();
        for element in self.0.children_with_tokens() {
            match element {
                ElementView::Token(token) if token.kind() == SyntaxKind::InKw => break,
                ElementView::Token(token) if token.kind() == SyntaxKind::Ident => {
                    names.push(LoopName {
                        name: Name::new(token),
                        ty: None,
                    });
                }
                ElementView::Token(_) => {}
                ElementView::Node(node) => {
                    if let (Some(ty), Some(last)) = (TypeRef::cast(node), names.last_mut()) {
                        last.ty = Some(ty);
                    }
                }
            }
        }
        names
    }

    pub fn iterable(&self) -> Option<Iterable<'p>> {
        child(&self.0)
    }

    pub fn range(&self) -> Range<u32> {
        let start = self.0.range().start;
        let mut end = start;
        let mut after_in = false;
        for element in self.0.children_with_tokens() {
            match element {
                ElementView::Node(node) if node.kind() == SyntaxKind::Block => break,
                ElementView::Node(node) => {
                    end = node.range().end;
                    if Iterable::cast(node).is_some() {
                        break;
                    }
                }
                ElementView::Token(token) => match token.kind() {
                    SyntaxKind::Colon | SyntaxKind::IntoKw | SyntaxKind::Newline if after_in => {
                        break;
                    }
                    kind if kind.is_trivia() => {}
                    kind => {
                        after_in |= kind == SyntaxKind::InKw;
                        end = token.range().end;
                    }
                },
            }
        }
        start..end
    }
}
