use super::ast_node::AstNode;
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
