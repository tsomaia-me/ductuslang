mod support;

use symphonyscript::{Code, Compiler, Diagnostic};
use symphonyscript_fixtures::{Rows, covered, vocabulary};

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Loop,
    Accumulator,
    Let,
}

#[derive(Clone, Copy, PartialEq)]
enum Source {
    TopLevel,
    Child,
    Parameter,
    LoopName,
    Accumulator,
}

const KINDS: [Kind; 3] = [Kind::Loop, Kind::Accumulator, Kind::Let];

const SOURCES: [Source; 5] = [
    Source::TopLevel,
    Source::Child,
    Source::Parameter,
    Source::LoopName,
    Source::Accumulator,
];

const ITERABLES: [&str; 2] = ["[1, 2]", "0..2"];

fn possible(kind: Kind, source: Source, annotated: bool, iterable: usize) -> bool {
    let two_names = source == Source::LoopName && kind == Kind::Loop;
    !(kind == Kind::Let && (annotated || source == Source::Child)
        || kind == Kind::Accumulator && source == Source::Accumulator
        || two_names && iterable == 1)
}

fn for_expression(
    kind: Kind,
    source: Source,
    annotated: bool,
    iterable: &str,
    name: &str,
) -> String {
    let typed = |text: &str| {
        if annotated {
            format!("{text}: i32")
        } else {
            text.to_string()
        }
    };
    let mut names = vec!["x".to_string()];
    let mut accumulator = "acc".to_string();
    let mut local = "k".to_string();
    match kind {
        Kind::Loop => names[0] = typed(name),
        Kind::Accumulator => accumulator = typed(name),
        Kind::Let => local = name.to_string(),
    }
    match source {
        Source::LoopName if kind == Kind::Loop => names.insert(0, "t".to_string()),
        Source::LoopName => names[0] = "t".to_string(),
        Source::Accumulator => accumulator = "t".to_string(),
        Source::TopLevel | Source::Child | Source::Parameter => {}
    }
    let element = names
        .last()
        .map_or("x", |last| last.split(':').next().unwrap_or("x"));
    let total = accumulator.split(':').next().unwrap_or("acc").to_string();
    let head = format!(
        "for {} in {iterable} into {accumulator} from 0:",
        names.join(", ")
    );
    if kind == Kind::Let || source == Source::Parameter {
        return format!(
            "{head}\n    INDENT    let {local} = 2\n    INDENT    {total} + {local} + {element}\n"
        );
    }
    format!("{head} {total} + {element}\n")
}

fn program(kind: Kind, source: Source, annotated: bool, iterable: &str, name: &str) -> String {
    let for_text = for_expression(kind, source, annotated, iterable, name);
    match source {
        Source::TopLevel => format!(
            "let t = 1\nlet v = {}Clip c:\n    Note/(v + t)\n",
            for_text.replace("INDENT", "")
        ),
        Source::Child => format!(
            "Clip c:\n    Note as t\n    Note | velocity=({})\n",
            for_text.trim_end()
        ),
        Source::Parameter => format!(
            "fn f(t):\n    {}Clip c:\n    Note/(f(1))\n",
            for_text.replace("INDENT", "    ")
        ),
        Source::LoopName | Source::Accumulator => format!(
            "let v = {}Clip c:\n    Note/v\n",
            for_text.replace("INDENT", "")
        ),
    }
}

fn errors(compiler: &Compiler, text: &str) -> Vec<Diagnostic> {
    support::errors(compiler.check(&[support::source(text)]))
}

fn name_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (kind_at, kind) in KINDS.into_iter().enumerate() {
        for (source_at, source) in SOURCES.into_iter().enumerate() {
            for (annotated_at, annotated) in [false, true].into_iter().enumerate() {
                for (iterable_at, iterable) in ITERABLES.into_iter().enumerate() {
                    if !possible(kind, source, annotated, iterable_at)
                        || !rows.take(&[kind_at, source_at, annotated_at, iterable_at])
                    {
                        continue;
                    }
                    programs += 1;
                    let text = program(kind, source, annotated, iterable, "t");
                    let twin = program(kind, source, annotated, iterable, "u");
                    let code = match source {
                        Source::TopLevel | Source::Child | Source::Parameter => {
                            Code::ForVariableClash
                        }
                        Source::LoopName | Source::Accumulator => Code::DuplicateLocal,
                    };
                    let found = errors(&compiler, &text);
                    let twin_found = errors(&compiler, &twin);
                    let holds = twin_found.is_empty()
                        && found.iter().all(|diagnostic| {
                            let range = &diagnostic.span.range;
                            text.get(range.start as usize..range.end as usize) == Some("t")
                        })
                        && matches!(found.as_slice(), [diagnostic] if diagnostic.code == code);
                    if !holds {
                        failures.push(format!(
                            "{text}\n  draws {found:?}\n  its twin {twin}\n  draws {twin_found:?}\n"
                        ));
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn assert_none_fail((failures, programs): (Vec<String>, usize), expected: usize) {
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(6).cloned().collect::<String>()
    );
    assert_eq!(programs, expected);
}

#[test]
fn a_for_expressions_names_are_new_and_distinct_names_whatever_they_bind() {
    assert_none_fail(covered(name_rows), 15);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_for_expressions_names_are_new_and_distinct_names_whatever_they_bind() {
    assert_none_fail(name_rows(&Rows::Every), 42);
}

#[test]
fn the_names_of_the_spec_bind_and_their_repeats_are_rejected() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let held = "state held([]): i32[..10]\n";
    for (text, expected) in [
        (
            format!(
                "{held}state high = for x in held into kept from []:\n    let keep = x > 60\n    if keep then kept + [x] else kept\nClip c:\n    Note/(high[0])\n"
            ),
            vec![],
        ),
        (
            format!("{held}state v = for x in held into x from 0: x\nClip c:\n    Note/v\n"),
            vec![Code::DuplicateLocal],
        ),
        (
            "let v = for i, i in [1, 2] into s from 0: s + i\nClip c:\n    Note/v\n".to_string(),
            vec![Code::DuplicateLocal],
        ),
        (
            format!(
                "{held}state total: = for x in held into sum from 0: sum + x\nClip c:\n    Note/total\n"
            ),
            vec![Code::TypeSyntax],
        ),
        (
            "let v = for x: bool in [1, 2] into s from 0: s + 1\nClip c:\n    Note/v\n".to_string(),
            vec![Code::TypeMismatch],
        ),
        (
            "let v = for x: u8 in [1, 300] into s from 0: s + x\nClip c:\n    Note/v\n".to_string(),
            vec![Code::DoesNotFit],
        ),
        (
            "let v = for x: f32 in [1, 2] into s from 0: s + x\nClip c:\n    Note | velocity=v\n"
                .to_string(),
            vec![],
        ),
        (
            format!(
                "{held}state v = for x: i32 in held into s: i32[..4] from []: s + [x]\nClip c:\n    Note/(v[0])\n"
            ),
            vec![Code::CapacityExceeded],
        ),
        (
            format!(
                "{held}state v = for x: i32 in held into s: i32[..10] from []: s + [x]\nClip c:\n    Note/(v[0])\n"
            ),
            vec![],
        ),
    ] {
        let found: Vec<Code> = errors(&compiler, &text)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert_eq!(found, expected, "{text}");
    }
}

#[test]
fn a_block_let_is_visible_throughout_its_block_and_the_names_of_one_head_and_block_are_distinct() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let block = |lets: &str| {
        format!(
            "let v = for x in [3] into s from 0:\n{lets}    s + b\nClip q:\n    Note/([0][v])\n"
        )
    };
    let spanned = |text: &str, diagnostic: &Diagnostic| {
        text.get(diagnostic.span.range.start as usize..diagnostic.span.range.end as usize)
            .unwrap_or_default()
            .to_string()
    };
    for (text, expected, at) in [
        (
            block("    let a = x + 1\n    let b = a + 1\n"),
            Code::IndexOutOfRange,
            "v",
        ),
        (
            block("    let b = a + 1\n    let a = x + 1\n"),
            Code::IndexOutOfRange,
            "v",
        ),
        (
            block("    let a = b\n    let b = a\n"),
            Code::DeclarationCycle,
            "a",
        ),
        (
            block("    let b = 0\n    let b = 0\n"),
            Code::DuplicateLocal,
            "b",
        ),
        (
            "Clip q:\n    for i, i in [1, 2]:\n        Note\n".to_string(),
            Code::DuplicateLocal,
            "i",
        ),
        (
            "Clip q:\n    [for i, i in [1, 2]: Note]\n".to_string(),
            Code::DuplicateLocal,
            "i",
        ),
    ] {
        let found = errors(&compiler, &text);
        let holds = match found.as_slice() {
            [diagnostic] if diagnostic.code == expected && spanned(&text, diagnostic) == at => {
                expected != Code::IndexOutOfRange
                    || diagnostic.summary.starts_with("the index 5 is outside")
            }
            _ => false,
        };
        assert!(
            holds,
            "{text}\n  draws {found:?}, not one {expected:?} at {at:?}"
        );
    }
}
