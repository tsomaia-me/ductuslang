mod support;

use symphonyscript::{Code, Compiler, Diagnostic};
use symphonyscript_fixtures::{Rows, covered, spec_examples, vocabulary};

const HEADS: [(&str, &str, bool); 6] = [
    ("x", "60 + x", false),
    ("x: i32", "60 + x", false),
    ("i, x", "60 + i + x", true),
    ("i: i32, x", "60 + i + x", true),
    ("i, x: i32", "60 + i + x", true),
    ("i: i32, x: i32", "60 + i + x", true),
];

const ITERABLES: [(&str, bool); 2] = [("0..4", true), ("[1, 2]", false)];

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

fn one_at(found: &[Diagnostic], text: &str, code: Code, at: &str) -> bool {
    matches!(found, [diagnostic] if diagnostic.code == code && spanned(text, diagnostic) == at)
}

fn head_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (site, template) in SITES.iter().enumerate() {
        for (head, (names, body, two)) in HEADS.iter().enumerate() {
            for (iterable, (values, range)) in ITERABLES.iter().enumerate() {
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
                    one_at(&twin_found, &twin, Code::UnbuiltConstruct, &whole(&twin))
                        && one_at(&found, &text, Code::UnbuiltConstruct, &whole(&text))
                } else if *two && *range {
                    let second = names
                        .split(", ")
                        .nth(1)
                        .and_then(|name| name.split(':').next())
                        .unwrap_or_default();
                    twin_found.is_empty() && one_at(&found, &text, Code::IterableType, second)
                } else {
                    twin_found.is_empty() && found.is_empty()
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
fn a_loop_head_binds_one_or_two_annotated_names_at_every_site_and_a_range_binds_one() {
    assert_none_fail(covered(head_rows), 18);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_loop_head_binds_one_or_two_annotated_names_at_every_site_and_a_range_binds_one() {
    assert_none_fail(head_rows(&Rows::Every), 36);
}

#[test]
fn the_loop_heads_of_the_spec_bind_their_names_and_a_range_with_two_names_is_rejected() {
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
    for text in [example("7.8", "sym"), strum] {
        assert_eq!(support::parsed(&text).diagnostics, vec![], "{text}");
        let found = errors(&compiler, &text);
        assert!(found.is_empty(), "{text}\n  draws {found:?}");
    }
    let rejected = example("7.9", "sym-error");
    let found = errors(&compiler, &rejected);
    assert!(
        one_at(&found, &rejected, Code::IterableType, "k"),
        "{rejected}\n  draws {found:?}"
    );
}
