use std::ops::Range;

use crate::diagnostics::Diagnostic;
use crate::syntax::{ElementView, NodeView, SyntaxKind, Walk};

pub(super) struct Faults {
    marks: Vec<(u32, u32)>,
}

impl Faults {
    pub fn new(diagnostics: &[Diagnostic], detached: &[Range<u32>], root: &NodeView<'_>) -> Self {
        let key = |range: &Range<u32>| (range.start, range.end);
        let mut all: Vec<Range<u32>> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.span.range.clone())
            .filter(|range| !range.is_empty())
            .collect();
        all.sort_by_key(key);
        let mut unmarked = detached.to_vec();
        unmarked.sort_by_key(key);
        let mut unmarked = unmarked.into_iter().peekable();
        let mut ranges = Vec::with_capacity(all.len());
        for range in all {
            while unmarked.next_if(|skip| key(skip) < key(&range)).is_some() {}
            if unmarked.next_if(|skip| *skip == range).is_none() {
                ranges.push(range);
            }
        }
        ranges.extend(unclosed_brackets(root));
        ranges.sort_by_key(key);
        let mut furthest = 0;
        let marks = ranges
            .iter()
            .map(|range| {
                furthest = furthest.max(range.end.max(range.start + 1));
                (range.start, furthest)
            })
            .collect();
        Self { marks }
    }

    pub fn first_in(&self, range: Range<u32>) -> Option<u32> {
        let from = self
            .marks
            .partition_point(|(start, _)| *start < range.start);
        self.marks
            .get(from)
            .map(|(start, _)| *start)
            .filter(|start| *start < range.end)
    }

    pub fn touch(&self, range: Range<u32>) -> bool {
        let end = range.end.max(range.start + 1);
        let before = self.marks.partition_point(|(start, _)| *start < end);
        self.marks
            .iter()
            .take(before)
            .next_back()
            .is_some_and(|(_, reach)| *reach > range.start)
    }
}

fn unclosed_brackets(root: &NodeView<'_>) -> Vec<Range<u32>> {
    root.preorder()
        .filter_map(|event| match event {
            Walk::Enter(ElementView::Node(node)) => unclosed(&node),
            Walk::Enter(ElementView::Token(_)) | Walk::Leave(_) => None,
        })
        .collect()
}

fn unclosed(node: &NodeView<'_>) -> Option<Range<u32>> {
    let closer = match node.kind() {
        SyntaxKind::ParenExpr
        | SyntaxKind::ArgList
        | SyntaxKind::Conversion
        | SyntaxKind::DefaultArg => SyntaxKind::RParen,
        SyntaxKind::ArrayLit | SyntaxKind::IndexExpr => SyntaxKind::RBracket,
        SyntaxKind::History => SyntaxKind::RBrace,
        _ => return None,
    };
    let opener = match closer {
        SyntaxKind::RParen => SyntaxKind::LParen,
        SyntaxKind::RBracket => SyntaxKind::LBracket,
        _ => SyntaxKind::LBrace,
    };
    let mut opened = None;
    for element in node.children_with_tokens() {
        let ElementView::Token(token) = element else {
            continue;
        };
        if token.kind() == opener && opened.is_none() {
            opened = Some(token.range().start);
        } else if token.kind() == closer && opened.is_some() {
            return None;
        }
    }
    opened.map(|start| start..node.range().end.max(start + 1))
}
