use rowan::{NodeOrToken, TextSize};

use super::syntax_element::SyntaxElement;
use super::syntax_node::SyntaxNode;
use super::syntax_token::SyntaxToken;

#[derive(Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

pub(crate) fn first_token(element: SyntaxElement) -> Option<SyntaxToken> {
    edge_token(element, Direction::Forward)
}

pub(crate) fn last_token(element: SyntaxElement) -> Option<SyntaxToken> {
    edge_token(element, Direction::Backward)
}

pub(crate) fn next_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    step_token(token, Direction::Forward)
}

pub(crate) fn prev_token(token: &SyntaxToken) -> Option<SyntaxToken> {
    step_token(token, Direction::Backward)
}

fn edge_child(node: &SyntaxNode, direction: Direction) -> Option<SyntaxElement> {
    match direction {
        Direction::Forward => node.first_child_or_token(),
        Direction::Backward => node.last_child_or_token(),
    }
}

fn sibling(element: &SyntaxElement, direction: Direction) -> Option<SyntaxElement> {
    match direction {
        Direction::Forward => element.next_sibling_or_token(),
        Direction::Backward => element.prev_sibling_or_token(),
    }
}

fn edge_token(element: SyntaxElement, direction: Direction) -> Option<SyntaxToken> {
    let start = match element {
        NodeOrToken::Token(token) => return Some(token),
        NodeOrToken::Node(node) => node,
    };
    let mut current = start.clone();
    loop {
        match edge_child(&current, direction) {
            Some(NodeOrToken::Token(token)) => return Some(token),
            Some(NodeOrToken::Node(child)) => current = child,
            None => loop {
                if current == start {
                    return None;
                }
                match sibling(&NodeOrToken::Node(current.clone()), direction) {
                    Some(NodeOrToken::Token(token)) => return Some(token),
                    Some(NodeOrToken::Node(next)) => {
                        current = next;
                        break;
                    }
                    None => current = current.parent()?,
                }
            },
        }
    }
}

fn step_token(token: &SyntaxToken, direction: Direction) -> Option<SyntaxToken> {
    let mut current = NodeOrToken::Token(token.clone());
    loop {
        match sibling(&current, direction) {
            Some(next) => {
                if let Some(found) = edge_token(next.clone(), direction) {
                    return Some(found);
                }
                current = next;
            }
            None => current = NodeOrToken::Node(current.parent()?),
        }
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
