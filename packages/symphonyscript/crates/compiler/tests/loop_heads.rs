mod support;

use symphonyscript::{Code, Compiler, Diagnostic};
use symphonyscript_fixtures::{Rows, covered, spec_examples, vocabulary};

const HEADS: [(&str, &str, Option<&str>); 6] = [
    ("x", "60 + x", None),
    ("x: i32", "60 + x", Some("an annotated loop name")),
    ("i, x", "60 + i + x", Some("a loop head with two names")),
    (
        "i: i32, x",
        "60 + i + x",
        Some("a loop head with two names"),
    ),
    (
        "i, x: i32",
        "60 + i + x",
        Some("a loop head with two names"),
    ),
    (
        "i: i32, x: i32",
        "60 + i + x",
        Some("a loop head with two names"),
    ),
];

const ITERABLES: [&str; 2] = ["0..4", "[1, 2]"];

const SITES: [&str; 3] = [
    "Clip c:\n    H:\n        Note/(B)\n",
    "Clip c:\n    [H: Note/(B)]\n",
    "let v = H into s from 0: s + B\nClip c:\n    Note\n",
];

const FOR_EXPRESSION_SITE: usize = 2;

fn errors(compiler: &Compiler, text: &str) -> Vec<Diagnostic> {
    support::errors(compiler.check(&[support::source(text)]))
}

fn spanned<'t>(text: &'t str, diagnostic: &Diagnostic) -> &'t str {
    let range = &diagnostic.span.range;
    text.get(range.start as usize..range.end as usize)
        .unwrap_or_default()
}

fn unbuilt_at(found: &[Diagnostic], text: &str, construct: &str, summary: &str) -> bool {
    matches!(found, [diagnostic]
        if diagnostic.code == Code::UnbuiltConstruct
            && spanned(text, diagnostic) == construct
            && diagnostic.summary.contains(summary))
}

fn head_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (site, template) in SITES.iter().enumerate() {
        for (head, (names, body, construct)) in HEADS.iter().enumerate() {
            for (iterable, values) in ITERABLES.iter().enumerate() {
                if !rows.take(&[site, head, iterable]) {
                    continue;
                }
                programs += 1;
                let loop_head = format!("for {names} in {values}");
                let program =
                    |head: &str, body: &str| template.replace('H', head).replace('B', body);
                let text = program(&loop_head, body);
                let twin = program(&format!("for x in {values}"), "60 + x");
                let found = errors(&compiler, &text);
                let twin_found = errors(&compiler, &twin);
                let holds = if site == FOR_EXPRESSION_SITE {
                    let whole = |text: &str| {
                        text.trim_start_matches("let v = ")
                            .split('\n')
                            .next()
                            .unwrap_or_default()
                            .to_string()
                    };
                    let summary = "`for` expression (SPEC §5.10)";
                    unbuilt_at(&twin_found, &twin, &whole(&twin), summary)
                        && unbuilt_at(&found, &text, &whole(&text), summary)
                } else {
                    match construct {
                        None => twin_found.is_empty() && found.is_empty(),
                        Some(construct) => {
                            let summary = format!("{construct} (SPEC §7.9)");
                            twin_found.is_empty() && unbuilt_at(&found, &text, &loop_head, &summary)
                        }
                    }
                };
                if !holds {
                    failures.push(format!(
                        "{text}\n  draws {found:?}\n  its twin draws {twin_found:?}\n"
                    ));
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
fn a_loop_head_with_two_names_or_an_annotation_draws_unbuilt_construct_at_every_site() {
    assert_none_fail(covered(head_rows), 18);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_loop_head_with_two_names_or_an_annotation_draws_unbuilt_construct_at_every_site() {
    assert_none_fail(head_rows(&Rows::Every), 36);
}

#[test]
fn the_loop_heads_of_the_spec_parse_and_draw_unbuilt_construct_at_their_heads() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let example = |section: &str, tag: &str| {
        spec_examples()
            .into_iter()
            .find(|example| {
                example.section == section && example.tag == tag && example.ordinal == 1
            })
            .map(|example| example.text)
            .expect("the SPEC holds the example")
    };
    let strum = "let chord = [60, 64, 67]\n\nClip strum:\n    for i, p in chord:\n        Note/p | velocity=1.0 - i * 0.2\n".to_string();
    for (text, head) in [
        (example("7.8", "sym"), "for i, p in [60, 64, 67]"),
        (strum, "for i, p in chord"),
        (example("7.9", "sym-error"), "for i, k in 0..4"),
    ] {
        assert_eq!(support::parsed(&text).diagnostics, vec![], "{text}");
        let found = errors(&compiler, &text);
        assert!(
            unbuilt_at(
                &found,
                &text,
                head,
                "a loop head with two names (SPEC §7.9)"
            ),
            "{text}\n  draws {found:?}"
        );
    }
}
