#![allow(dead_code)]

use rowan::{NodeOrToken, WalkEvent};
use symphonyscript::syntax::{Parsed, SyntaxKind, SyntaxNode, parse};
use symphonyscript::{Code, Source};
use symphonyscript_fixtures::vocabulary;

pub fn source(text: &str) -> Source {
    Source {
        path: "test.sym".into(),
        text: text.into(),
    }
}

pub fn parsed(text: &str) -> Parsed {
    parse(&source(text), &vocabulary())
}

pub fn codes(text: &str) -> Vec<(Code, u32, u32)> {
    parsed(text)
        .diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.span.range.start,
                diagnostic.span.range.end,
            )
        })
        .collect()
}

pub fn tree(text: &str) -> String {
    sexp(&parsed(text).syntax())
}

pub fn clean_tree(text: &str) -> String {
    let result = parsed(text);
    assert_eq!(result.diagnostics, vec![], "{text:?}");
    sexp(&result.syntax())
}

pub fn sexp(node: &SyntaxNode) -> String {
    let mut out = String::new();
    let mut first = vec![true];
    for event in node.preorder_with_tokens() {
        match event {
            WalkEvent::Enter(element) => {
                let opens = element.as_node().is_some();
                let piece = match &element {
                    NodeOrToken::Node(child) => format!("{:?}[", child.kind()),
                    NodeOrToken::Token(token) => match token.kind() {
                        kind if kind.is_trivia() => continue,
                        SyntaxKind::Newline => "NL".to_string(),
                        SyntaxKind::Indent => ">".to_string(),
                        SyntaxKind::Dedent => "<".to_string(),
                        _ => token.text().to_string(),
                    },
                };
                if let Some(first) = first.last_mut() {
                    if !*first {
                        out.push(' ');
                    }
                    *first = false;
                }
                out.push_str(&piece);
                if opens {
                    first.push(true);
                }
            }
            WalkEvent::Leave(NodeOrToken::Node(_)) => {
                first.pop();
                out.push(']');
            }
            WalkEvent::Leave(NodeOrToken::Token(_)) => {}
        }
    }
    out
}
