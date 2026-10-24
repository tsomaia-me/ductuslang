use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use crate::syntax::{SyntaxKind, SyntaxNode, SyntaxToken};

pub(super) fn child<N: AstNode>(node: &SyntaxNode) -> Option<N> {
    node.children().find_map(N::cast)
}

pub(super) fn children<N: AstNode>(node: &SyntaxNode) -> impl Iterator<Item = N> + use<N> {
    node.children().filter_map(N::cast)
}

pub(super) fn tokens(
    node: &SyntaxNode,
    kind: SyntaxKind,
) -> impl Iterator<Item = SyntaxToken> + use<> {
    node.children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(move |token| token.kind() == kind)
}

pub(super) fn token(node: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxToken> {
    tokens(node, kind).next()
}

pub(super) fn name_at(node: &SyntaxNode, position: usize) -> Option<Name> {
    tokens(node, SyntaxKind::Ident).nth(position).map(Name::new)
}

pub(super) fn expr_before(node: &SyntaxNode, marker: impl Fn(SyntaxKind) -> bool) -> Option<Expr> {
    for element in node.children_with_tokens() {
        match element {
            rowan::NodeOrToken::Token(token) if marker(token.kind()) => return None,
            rowan::NodeOrToken::Node(child) => {
                if let Some(expr) = Expr::cast(child) {
                    return Some(expr);
                }
            }
            rowan::NodeOrToken::Token(_) => {}
        }
    }
    None
}

pub(super) fn expr_after(
    node: &SyntaxNode,
    marker: impl Fn(SyntaxKind) -> bool,
    stop: impl Fn(SyntaxKind) -> bool,
) -> Option<Expr> {
    let mut started = false;
    for element in node.children_with_tokens() {
        match element {
            rowan::NodeOrToken::Token(token) => {
                if !started && marker(token.kind()) {
                    started = true;
                } else if started && stop(token.kind()) {
                    return None;
                }
            }
            rowan::NodeOrToken::Node(child) if started => {
                if let Some(expr) = Expr::cast(child) {
                    return Some(expr);
                }
            }
            rowan::NodeOrToken::Node(_) => {}
        }
    }
    None
}
