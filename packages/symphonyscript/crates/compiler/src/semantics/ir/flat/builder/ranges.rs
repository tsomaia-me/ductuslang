use crate::syntax::ast::{self, AstNode};
use crate::syntax::{ElementView, NodeView, SyntaxKind};

fn significant(kind: SyntaxKind) -> bool {
    !kind.is_trivia()
        && !matches!(
            kind,
            SyntaxKind::Newline | SyntaxKind::Indent | SyntaxKind::Dedent
        )
}

pub(super) fn head(node: &NodeView<'_>) -> (u32, u32) {
    let range = node.range();
    let mut end = range.start;
    for element in node.children_with_tokens() {
        match element {
            ElementView::Node(child) if child.kind() == SyntaxKind::Body => break,
            ElementView::Node(child) => {
                let mut token = child.last_token();
                while let Some(current) = token {
                    if current.range().start < child.range().start {
                        token = None;
                        break;
                    }
                    if significant(current.kind()) {
                        break;
                    }
                    token = current.prev_token();
                }
                if let Some(token) = token {
                    end = end.max(token.range().end);
                }
            }
            ElementView::Token(token) => {
                if token.kind() == SyntaxKind::Newline {
                    break;
                }
                if significant(token.kind()) {
                    end = end.max(token.range().end);
                }
            }
        }
    }
    (range.start, end.max(range.start))
}

pub(super) fn trimmed(node: &NodeView<'_>) -> (u32, u32) {
    let start = node.range().start;
    let mut token = node.last_token();
    while let Some(current) = token {
        if current.range().start < start {
            break;
        }
        if significant(current.kind()) {
            return (start, current.range().end);
        }
        token = current.prev_token();
    }
    let range = node.range();
    (range.start, range.end)
}

pub(super) fn line_end(item: &NodeView<'_>) -> u32 {
    let range = item.range();
    let mut token = item.first_token();
    while let Some(current) = token {
        if matches!(current.kind(), SyntaxKind::Newline | SyntaxKind::Indent)
            || current.range().start >= range.end
        {
            return range.end.min(current.range().start);
        }
        token = current.next_token();
    }
    range.end
}

pub(super) fn brackets(node: &ast::CellDecl<'_>) -> Option<(u32, u32)> {
    let mut tokens = node
        .syntax()
        .children_with_tokens()
        .filter_map(ElementView::into_token);
    let open = tokens.find(|token| token.kind() == SyntaxKind::LBracket)?;
    let close = tokens.find(|token| token.kind() == SyntaxKind::RBracket)?;
    Some((open.range().start, close.range().end))
}

pub(super) fn annotation_end(node: &ast::CellDecl<'_>) -> u32 {
    let eq = node
        .syntax()
        .children_with_tokens()
        .filter_map(ElementView::into_token)
        .find(|token| token.kind() == SyntaxKind::Eq)
        .map(|token| token.range().start);
    let clock = node.clock().map(|clock| clock.syntax().range().start);
    clock.or(eq).unwrap_or(node.syntax().range().end)
}

pub(super) fn skipped_block(node: &NodeView<'_>) -> bool {
    node.kind() == SyntaxKind::ErrorNode
        && node
            .children_with_tokens()
            .find_map(|element| match element {
                ElementView::Token(token) if token.kind().is_trivia() => None,
                ElementView::Token(token) => Some(token.kind() == SyntaxKind::Indent),
                ElementView::Node(_) => Some(false),
            })
            .unwrap_or(false)
}
