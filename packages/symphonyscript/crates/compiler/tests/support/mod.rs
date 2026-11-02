#![allow(dead_code, unused_imports)]

use symphonyscript::syntax::{ElementView, NodeView, Parsed, SyntaxKind, Walk, parse};
use symphonyscript::{Code, Compiler, Diagnostic, Severity, Source};
use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::vocabulary;
pub use symphonyscript_fixtures::{Orders, signature_vocabulary, to_array_vocabulary};

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
    sexp(&parsed(text).root())
}

pub fn clean_tree(text: &str) -> String {
    let result = parsed(text);
    assert_eq!(result.diagnostics, vec![], "{text:?}");
    sexp(&result.root())
}

pub fn sexp(node: &NodeView<'_>) -> String {
    let mut out = String::new();
    let mut first = vec![true];
    for event in node.preorder() {
        match event {
            Walk::Enter(element) => {
                let opens = element.as_node().is_some();
                let piece = match &element {
                    ElementView::Node(child) => format!("{:?}[", child.kind()),
                    ElementView::Token(token) => match token.kind() {
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
            Walk::Leave(ElementView::Node(_)) => {
                first.pop();
                out.push(']');
            }
            Walk::Leave(ElementView::Token(_)) => {}
        }
    }
    out
}

pub fn checked(text: &str) -> Vec<Diagnostic> {
    errors(checked_all(text))
}

pub fn checked_all(text: &str) -> Vec<Diagnostic> {
    checked_files_all(&[("test.sym", text)])
}

pub fn errors(diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .collect()
}

pub fn checked_with(vocabulary: &Vocabulary, text: &str) -> Vec<Diagnostic> {
    let compiler = Compiler::new(vocabulary).expect("the vocabulary compiles");
    errors(compiler.check(&[source(text)]))
}

pub fn checked_files(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    errors(checked_files_all(files))
}

thread_local! {
    static EXAMPLE: Compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
}

pub fn checked_files_all(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let sources: Vec<Source> = files
        .iter()
        .map(|(path, text)| Source {
            path: (*path).into(),
            text: (*text).into(),
        })
        .collect();
    EXAMPLE.with(|compiler| compiler.check(&sources))
}

pub fn check_codes(text: &str) -> Vec<(Code, u32, u32)> {
    checked(text)
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

pub fn at(text: &str, needle: &str, occurrence: usize) -> (u32, u32) {
    let mut from = 0;
    let mut found = None;
    for _ in 0..=occurrence {
        let index = text[from..].find(needle).map(|offset| from + offset);
        found = index;
        from = index.expect("the needle occurs") + needle.len();
    }
    let start = found.expect("the needle occurs");
    (start as u32, (start + needle.len()) as u32)
}

pub fn expect_codes(text: &str, expected: &[(Code, &str, usize)]) {
    let expected: Vec<(Code, u32, u32)> = expected
        .iter()
        .map(|(code, needle, occurrence)| {
            let (start, end) = at(text, needle, *occurrence);
            (*code, start, end)
        })
        .collect();
    assert_eq!(check_codes(text), expected, "{text}");
}

pub fn permutations<T: Copy>(items: &[T]) -> Vec<Vec<T>> {
    Orders::Every.of(items)
}

pub type Row<'a> = (&'a [&'a str], &'a [(Code, &'a str)]);

pub fn error_texts(compiler: &Compiler, text: &str) -> Vec<(Code, String)> {
    let mut found: Vec<(Code, String)> = errors(compiler.check(&[source(text)]))
        .iter()
        .map(|diagnostic| {
            let range = &diagnostic.span.range;
            let spanned = text
                .get(range.start as usize..range.end as usize)
                .unwrap_or_default();
            (diagnostic.code, spanned.to_string())
        })
        .collect();
    found.sort();
    found
}

pub fn assert_rows_in_every_order(vocabulary: &Vocabulary, rows: &[Row]) {
    assert_rows_in_orders(Orders::Every, vocabulary, rows);
}

pub fn assert_rows_in_orders(orders: Orders, vocabulary: &Vocabulary, rows: &[Row]) {
    let compiler = Compiler::new(vocabulary).expect("the vocabulary compiles");
    assert_rows_by(orders, &compiler, rows);
}

pub fn assert_rows_by(orders: Orders, compiler: &Compiler, rows: &[Row]) {
    for (items, expected) in rows {
        let mut expected: Vec<(Code, String)> = expected
            .iter()
            .map(|(code, text)| (*code, (*text).to_string()))
            .collect();
        expected.sort();
        for order in orders.of(items) {
            let text = order.concat();
            assert_eq!(error_texts(compiler, &text), expected, "{text}");
        }
    }
}

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn spanned(file: &str, range: &std::ops::Range<u32>) -> String {
    format!(
        "\"file\": {}, \"span\": [{}, {}]",
        quoted(file),
        range.start,
        range.end
    )
}

pub fn pinned_row(program: &str, diagnostics: &[Diagnostic]) -> String {
    let drawn: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| {
            let related: Vec<String> = diagnostic
                .related
                .iter()
                .map(|related| {
                    format!(
                        "{{{}, \"label\": {}}}",
                        spanned(&related.span.file, &related.span.range),
                        quoted(&related.label)
                    )
                })
                .collect();
            format!(
                "{{\"code\": {}, {}, \"summary\": {}, \"help\": {}, \"related\": [{}]}}",
                quoted(diagnostic.code.as_str()),
                spanned(&diagnostic.span.file, &diagnostic.span.range),
                quoted(&diagnostic.summary),
                diagnostic
                    .help
                    .as_deref()
                    .map_or("null".to_string(), quoted),
                related.join(", ")
            )
        })
        .collect();
    format!(
        "pinned-row {{\"program\": {}, \"diagnostics\": [{}]}}",
        quoted(program),
        drawn.join(", ")
    )
}

pub fn failed(
    program: &str,
    diagnostics: &[Diagnostic],
    message: impl std::fmt::Display,
) -> String {
    format!("{}\n{message}", pinned_row(program, diagnostics))
}

pub fn failed_row_with(
    compiler: &Compiler,
    program: &str,
    message: impl std::fmt::Display,
) -> String {
    failed(program, &compiler.check(&[source(program)]), message)
}

pub fn failed_row(program: &str, message: impl std::fmt::Display) -> String {
    EXAMPLE.with(|compiler| failed_row_with(compiler, program, message))
}

pub fn assert_no_failure(failures: &[String]) {
    for failure in failures {
        println!("{failure}");
    }
    assert!(failures.is_empty(), "{} rows fail", failures.len());
}

pub fn assert_none_fail((failures, programs): (Vec<String>, usize), expected: usize) {
    for failure in &failures {
        println!("{failure}");
    }
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail",
        failures.len()
    );
    assert_eq!(programs, expected);
}
