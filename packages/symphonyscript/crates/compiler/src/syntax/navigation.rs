use rowan::{NodeOrToken, TextSize};

use super::syntax_element::SyntaxElement;
use super::syntax_node::SyntaxNode;
use super::syntax_token::SyntaxToken;

pub(crate) fn first_token(element: SyntaxElement) -> Option<SyntaxToken> {
    let mut current = element;
    loop {
        match current {
            NodeOrToken::Token(token) => return Some(token),
            NodeOrToken::Node(node) => current = node.first_child_or_token()?,
        }
    }
}

pub(crate) fn last_token(element: SyntaxElement) -> Option<SyntaxToken> {
    let mut current = element;
    loop {
        match current {
            NodeOrToken::Token(token) => return Some(token),
            NodeOrToken::Node(node) => current = node.last_child_or_token()?,
        }
    }
}

pub(crate) fn next_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    match token.next_sibling_or_token() {
        Some(element) => first_token(element),
        None => token
            .parent()
            .into_iter()
            .flat_map(|parent| parent.ancestors())
            .find_map(|node| node.next_sibling_or_token())
            .and_then(first_token),
    }
}

pub(crate) fn prev_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    match token.prev_sibling_or_token() {
        Some(element) => last_token(element),
        None => token
            .parent()
            .into_iter()
            .flat_map(|parent| parent.ancestors())
            .find_map(|node| node.prev_sibling_or_token())
            .and_then(last_token),
    }
}

pub(crate) fn tokens_at_offset(root: &SyntaxNode, offset: TextSize) -> Vec<SyntaxToken> {
    let mut found = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(node) = pending.pop() {
        for child in node.children_with_tokens() {
            let range = child.text_range();
            if range.is_empty() || offset < range.start() || offset > range.end() {
                continue;
            }
            match child {
                NodeOrToken::Token(token) => found.push(token),
                NodeOrToken::Node(inner) => pending.push(inner),
            }
        }
    }
    found.sort_by_key(|token| token.text_range().start());
    found
}
