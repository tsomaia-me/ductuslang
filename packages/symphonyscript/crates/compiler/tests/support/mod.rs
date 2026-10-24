#![allow(dead_code)]

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
    write(node, &mut out);
    out
}

fn write(node: &SyntaxNode, out: &mut String) {
    out.push_str(&format!("{:?}[", node.kind()));
    let mut first = true;
    for element in node.children_with_tokens() {
        let piece = if let Some(child) = element.as_node() {
            let mut inner = String::new();
            write(child, &mut inner);
            inner
        } else if let Some(token) = element.as_token() {
            match token.kind() {
                kind if kind.is_trivia() => continue,
                SyntaxKind::Newline => "NL".to_string(),
                SyntaxKind::Indent => ">".to_string(),
                SyntaxKind::Dedent => "<".to_string(),
                _ => token.text().to_string(),
            }
        } else {
            continue;
        };
        if !first {
            out.push(' ');
        }
        first = false;
        out.push_str(&piece);
    }
    out.push(']');
}
