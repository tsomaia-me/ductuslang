#![allow(dead_code)]

use symphonyscript::syntax::{ElementView, NodeView, Parsed, SyntaxKind, Walk, parse};
use symphonyscript::{Code, Compiler, Diagnostic, Severity, Source};
use symphonyscript_core::{
    Bounded, Count, NativeArg, NativeOut, NativeSignature, NativeType, Ty, Value, Vocabulary,
};
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

pub fn checked_files_all(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let sources: Vec<Source> = files
        .iter()
        .map(|(path, text)| Source {
            path: (*path).into(),
            text: (*text).into(),
        })
        .collect();
    compiler.check(&sources)
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
    let mut found = Vec::new();
    let mut order: Vec<usize> = (0..items.len()).collect();
    let mut counters = vec![0; items.len()];
    found.push(order.iter().map(|index| items[*index]).collect());
    let mut level = 0;
    while level < items.len() {
        if counters[level] < level {
            if level % 2 == 0 {
                order.swap(0, level);
            } else {
                order.swap(counters[level], level);
            }
            found.push(order.iter().map(|index| items[*index]).collect());
            counters[level] += 1;
            level = 0;
        } else {
            counters[level] = 0;
            level += 1;
        }
    }
    found
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
    let compiler = Compiler::new(vocabulary).expect("the vocabulary compiles");
    for (items, expected) in rows {
        let mut expected: Vec<(Code, String)> = expected
            .iter()
            .map(|(code, text)| (*code, (*text).to_string()))
            .collect();
        expected.sort();
        for order in permutations(items) {
            let text = order.concat();
            assert_eq!(error_texts(&compiler, &text), expected, "{text}");
        }
    }
}

pub fn signature_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("signatures");
    let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
    let clip = v.node("Clip");
    let note = v.node("Note");
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any)
        .attr::<[u8; 3]>("fixed")
        .default([0, 0, 0])
        .attr::<Bounded<u8, 8>>("data")
        .default(Bounded::default())
        .attr::<Bounded<[i32; 2], 2>>("rows")
        .default(Bounded::default())
        .attr::<Bounded<i32, 3>>("three")
        .default(Bounded::default());
    v.kind(note)
        .attr::<i32>("pitch")
        .default(60)
        .default_attr()
        .attr::<f64>("length")
        .default(0.25)
        .family(&beats, "beats")
        .attr::<u8>("level")
        .default(0);
    v.native(
        "plain",
        NativeSignature::new([NativeType::Value(Ty::I32)], NativeType::Value(Ty::I32)),
        first,
    );
    v.native(
        "half",
        NativeSignature::new([NativeType::Value(Ty::F64)], NativeType::Value(Ty::F64)),
        half,
    );
    v.native(
        "cap4",
        NativeSignature::new(
            [NativeType::Value(Ty::I32.variable(4))],
            NativeType::Value(Ty::I32),
        ),
        first,
    );
    v.native(
        "fixed3",
        NativeSignature::new(
            [NativeType::Value(Ty::U8.array(3))],
            NativeType::Value(Ty::I32),
        ),
        first,
    );
    v.native(
        "shrink",
        NativeSignature::new(
            [NativeType::Open(Ty::I32.array(3))],
            NativeType::CapacityOf(Ty::I32.array(2), 0),
        ),
        shrink,
    );
    v.native(
        "squeeze",
        NativeSignature::new(
            [NativeType::Value(Ty::I32.variable(4))],
            NativeType::CapacityOf(Ty::I32, 0),
        ),
        shrink,
    );
    v.build().expect("the signature vocabulary builds")
}

fn first(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let value = match args.first() {
        Some(NativeArg::Value(value)) => Some(*value),
        Some(NativeArg::Elements(items)) => items.first().copied(),
        Some(NativeArg::Text(_)) | None => None,
    };
    out.set(match value {
        Some(Value::U8(byte)) => Value::I32(i32::from(byte)),
        Some(value) => value,
        None => Value::I32(0),
    });
}

fn half(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    let value = match args.first() {
        Some(NativeArg::Value(Value::F64(value))) => *value,
        Some(_) | None => 0.0,
    };
    out.set(Value::F64(value / 2.0));
}

fn shrink(args: &[NativeArg<'_>], out: &mut NativeOut<'_>) {
    if let Some(NativeArg::Elements(items)) = args.first() {
        for item in *items {
            if !out.push(*item) {
                break;
            }
        }
    }
}
