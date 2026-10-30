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
    "let v = H into s from 0: s + B\nClip c:\n    Note/v\n",
];

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
                let holds = if *two && *range {
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

fn outside(found: &[Diagnostic]) -> Vec<i128> {
    let mut indexes: Vec<i128> = found
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::IndexOutOfRange)
        .filter_map(|diagnostic| {
            diagnostic
                .summary
                .strip_prefix("the index ")
                .and_then(|rest| rest.split(' ').next())
                .and_then(|index| index.parse().ok())
        })
        .collect();
    indexes.sort_unstable();
    indexes
}

fn only_outside(found: &[Diagnostic], indexes: &[i128]) -> bool {
    found
        .iter()
        .all(|diagnostic| diagnostic.code == Code::IndexOutOfRange)
        && outside(found) == indexes
}

#[test]
fn two_names_bind_each_elements_position_and_the_element_and_the_position_is_flexible() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let chord = "let chord = [60, 64, 67]\n";
    for (text, indexes) in [
        (
            "let v = for i, x in [3, 5, 7] into s from 0: s + i * x\nClip k:\n    Note/([0][v])\n"
                .to_string(),
            vec![19],
        ),
        (
            format!(
                "{chord}Clip strum:\n    for i, p in chord:\n        Note/([0][p - chord[i]]) | velocity=1.0 - i * 0.2\n"
            ),
            vec![],
        ),
        (
            format!(
                "{chord}Clip strum:\n    for i, p in chord:\n        Note/([0][p - chord[0]])\n"
            ),
            vec![4, 7],
        ),
        (
            format!("{chord}Clip strum:\n    [for i, p in chord: Note/([0][i + 1])]\n"),
            vec![1, 2, 3],
        ),
        (
            format!(
                "{chord}Clip strum:\n    for i, p in chord:\n        Note/(60 + i) | velocity=1.0 - i * 0.2\n"
            ),
            vec![],
        ),
    ] {
        let found = errors(&compiler, &text);
        assert!(only_outside(&found, &indexes), "{text}\n  draws {found:?}");
    }
}

#[test]
fn a_placement_for_runs_its_body_once_per_element_with_compile_time_names_and_an_empty_range_runs_none()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for (text, indexes) in [
        (
            "Clip c:\n    for i in 0..3:\n        Note/([0][i])\n",
            vec![1, 2],
        ),
        (
            "Clip c:\n    for x in [4, 2, 6]:\n        Note/([0][x])\n",
            vec![2, 4, 6],
        ),
        (
            "Clip c:\n    for i in 1..2:\n        Note/([0][5])\n",
            vec![5],
        ),
        (
            "Clip c:\n    for i in 2..2:\n        Note/([0][5])\n",
            vec![],
        ),
        (
            "Clip c:\n    for i in 2..1:\n        Note/([0][5])\n",
            vec![],
        ),
        (
            "Clip c:\n    for i in 3..0:\n        Note/([0][5])\n",
            vec![],
        ),
    ] {
        let found = errors(&compiler, text);
        assert!(only_outside(&found, &indexes), "{text}\n  draws {found:?}");
    }
}

struct Elements {
    compile_time: &'static str,
    runtime: Option<&'static str>,
    reader: &'static str,
}

const CELLS: &str = "input c = 3\ninput f = 3.0\ninput b = true\ninput w = Wave::Saw\nClip verse:\n    Note as hook /3\n    Note as lead /5\n";

const ARRAYS: [Elements; 9] = [
    Elements {
        compile_time: "[3, 5]",
        runtime: Some("[c, 5]"),
        reader: "x",
    },
    Elements {
        compile_time: "[3.0, 5.0]",
        runtime: Some("[f, 5.0]"),
        reader: "i32(x)",
    },
    Elements {
        compile_time: "[true, false]",
        runtime: Some("[b, false]"),
        reader: "if x then 3 else 5",
    },
    Elements {
        compile_time: "[Wave::Saw, Wave::Sine]",
        runtime: Some("[w, Wave::Sine]"),
        reader: "if x is Wave::Saw then 3 else 5",
    },
    Elements {
        compile_time: "[\"a\", \"b\"]",
        runtime: None,
        reader: "if x is \"a\" then 3 else 5",
    },
    Elements {
        compile_time: "[verse.hook, verse.lead]",
        runtime: None,
        reader: "x.pitch",
    },
    Elements {
        compile_time: "[[3, 9], [5, 9]]",
        runtime: Some("[[c, 9], [5, 9]]"),
        reader: "x[0]",
    },
    Elements {
        compile_time: "[3] + [5]",
        runtime: Some("[c] + [5]"),
        reader: "x",
    },
    Elements {
        compile_time: "[[3, 9]] + [[5, 9]]",
        runtime: Some("[[c, 9]] + [[5, 9]]"),
        reader: "x[0]",
    },
];

const NOT_ITERABLE: [&str; 4] = ["5", "\"ab\"", "true", "verse.hook"];

fn iterating(site: usize, iterable: &str, reader: &str) -> String {
    let text = match site {
        0 => format!(
            "let v = for x in {iterable} into s from 0: s * 10 + ({reader})\nClip k:\n    Note/([0][v])\n"
        ),
        1 => format!("Clip k:\n    for x in {iterable}:\n        Note/([0][{reader}])\n"),
        _ => format!("Clip k:\n    [for x in {iterable}: Note/([0][{reader}])]\n"),
    };
    format!("{CELLS}{text}")
}

fn iterable_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for kind in 0..ARRAYS.len() + NOT_ITERABLE.len() {
        for site in 0..3 {
            for runtime in [false, true] {
                let iterable = match (ARRAYS.get(kind), runtime) {
                    (Some(elements), false) => elements.compile_time,
                    (Some(elements), true) if site == 0 => match elements.runtime {
                        Some(runtime) => runtime,
                        None => continue,
                    },
                    (Some(_), true) => continue,
                    (None, false) => NOT_ITERABLE[kind - ARRAYS.len()],
                    (None, true) => continue,
                };
                if !rows.take(&[kind, site, usize::from(runtime)]) {
                    continue;
                }
                programs += 1;
                let reader = ARRAYS.get(kind).map_or("0", |elements| elements.reader);
                let text = iterating(site, iterable, reader);
                let found = errors(&compiler, &text);
                let holds = match (ARRAYS.get(kind), runtime, site) {
                    (Some(_), true, _) => found.is_empty(),
                    (Some(_), false, 0) => only_outside(&found, &[35]),
                    (Some(_), false, _) => only_outside(&found, &[3, 5]),
                    (None, _, _) => one_at(&found, &text, Code::IterableType, iterable),
                };
                if !holds {
                    failures.push(format!("{text}\n  draws {found:?}\n"));
                }
            }
        }
    }
    (failures, programs)
}

#[test]
fn every_array_iterates_element_by_element_at_every_site_and_any_other_iterable_draws_iterable_type()
 {
    assert_none_fail(covered(iterable_rows), 44);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_every_array_iterates_element_by_element_at_every_site_and_any_other_iterable_draws_iterable_type()
 {
    assert_none_fail(iterable_rows(&Rows::Every), 46);
}
