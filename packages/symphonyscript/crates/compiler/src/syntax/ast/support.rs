use super::ast_node::AstNode;
use super::expr::Expr;
use super::name::Name;
use crate::syntax::{ElementView, NodeView, SyntaxKind, TokenView};

pub(super) fn child<'p, N: AstNode<'p>>(node: &NodeView<'p>) -> Option<N> {
    node.children().find_map(N::cast)
}

pub(super) fn children<'p, N: AstNode<'p>>(
    node: &NodeView<'p>,
) -> impl Iterator<Item = N> + use<'p, N> {
    node.children().filter_map(N::cast)
}

pub(super) fn tokens<'p>(
    node: &NodeView<'p>,
    kind: SyntaxKind,
) -> impl Iterator<Item = TokenView<'p>> + use<'p> {
    node.children_with_tokens()
        .filter_map(ElementView::into_token)
        .filter(move |token| token.kind() == kind)
}

pub(super) fn token<'p>(node: &NodeView<'p>, kind: SyntaxKind) -> Option<TokenView<'p>> {
    tokens(node, kind).next()
}

pub(super) fn name_at<'p>(node: &NodeView<'p>, position: usize) -> Option<Name<'p>> {
    tokens(node, SyntaxKind::Ident).nth(position).map(Name::new)
}

pub(super) fn expr_before<'p>(
    node: &NodeView<'p>,
    marker: impl Fn(SyntaxKind) -> bool,
) -> Option<Expr<'p>> {
    for element in node.children_with_tokens() {
        match element {
            ElementView::Token(token) if marker(token.kind()) => return None,
            ElementView::Node(child) => {
                if let Some(expr) = Expr::cast(child) {
                    return Some(expr);
                }
            }
            ElementView::Token(_) => {}
        }
    }
    None
}

pub(super) fn expr_after<'p>(
    node: &NodeView<'p>,
    marker: impl Fn(SyntaxKind) -> bool,
    stop: impl Fn(SyntaxKind) -> bool,
) -> Option<Expr<'p>> {
    let mut started = false;
    for element in node.children_with_tokens() {
        match element {
            ElementView::Token(token) => {
                if !started && marker(token.kind()) {
                    started = true;
                } else if started && stop(token.kind()) {
                    return None;
                }
            }
            ElementView::Node(child) if started => {
                if let Some(expr) = Expr::cast(child) {
                    return Some(expr);
                }
            }
            ElementView::Node(_) => {}
        }
    }
    None
}
