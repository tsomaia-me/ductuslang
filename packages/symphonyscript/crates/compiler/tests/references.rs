mod support;

use symphonyscript::{Code, Compiler, Diagnostic};
use symphonyscript_fixtures::{Rows, covered, vocabulary};

const TARGETS: &str =
    "Clip verse:\n    Note as hook /65\nClip chorus:\n    Plays as back | transpose=3: verse\n";

const REFERENCES: [(&str, &str, i128); 2] =
    [("verse.hook", "pitch", 64), ("chorus.back", "transpose", 2)];

const ACCESSES: [(&str, &str); 7] = [
    ("", "R.A"),
    ("let r = R\n", "r.A"),
    ("let rs = [R]\n", "rs[0].A"),
    ("fn f(x): x.A\n", "f(R)"),
    ("let t = for x in [R] into s from 0: s + x.A\n", "t"),
    (
        "fn f(x, n): if n is 0 then x.A else f(x, n - 1)\n",
        "f(R, 1)",
    ),
    ("placement", ""),
];

const REFUSALS: [(&str, Code); 4] = [
    ("input s = R\nClip c:\n    Note\n", Code::InputValue),
    ("state s = R\nClip c:\n    Note\n", Code::StateValue),
    ("Clip c:\n    Note | velocity=R\n", Code::TypeMismatch),
    (
        "input flag = false\nlet r = if flag then R else R\nClip c:\n    Note/(r.A)\n",
        Code::ReactiveNodeReference,
    ),
];

fn errors(compiler: &Compiler, text: &str) -> Vec<Diagnostic> {
    support::errors(compiler.check(&[support::source(text)]))
}

fn reads_one(found: &[Diagnostic]) -> bool {
    matches!(found, [diagnostic]
        if diagnostic.code == Code::IndexOutOfRange
            && diagnostic.summary.starts_with("the index 1 is outside"))
}

fn ordered(targets_first: bool, rest: &str) -> String {
    if targets_first {
        format!("{TARGETS}{rest}")
    } else {
        format!("{rest}{TARGETS}")
    }
}

fn reference_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (at, (prefix, read)) in ACCESSES.iter().enumerate() {
        for (kind, (reference, attribute, offset)) in REFERENCES.iter().enumerate() {
            for (order, targets_first) in [true, false].into_iter().enumerate() {
                if !rows.take(&[at, kind, order]) {
                    continue;
                }
                programs += 1;
                let fill = |text: &str| text.replace('R', reference).replace('A', attribute);
                let rest = if *prefix == "placement" {
                    fill(&format!(
                        "Clip c:\n    for x in [R]:\n        Note/([0][x.A - {offset}])\n"
                    ))
                } else {
                    format!(
                        "{}Clip c:\n    Note/([0][{} - {offset}])\n",
                        fill(prefix),
                        fill(read)
                    )
                };
                let text = ordered(targets_first, &rest);
                let twin = ordered(
                    targets_first,
                    &fill(&format!("Clip c:\n    Note/([0][R.A - {offset}])\n")),
                );
                let found = errors(&compiler, &text);
                let twin_found = errors(&compiler, &twin);
                if !(reads_one(&found) && reads_one(&twin_found)) {
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
fn a_reference_to_a_node_or_a_wire_reads_its_attribute_in_every_position_it_stands_in() {
    assert_none_fail(covered(reference_rows), 14);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_reference_to_a_node_or_a_wire_reads_its_attribute_in_every_position_it_stands_in() {
    assert_none_fail(reference_rows(&Rows::Every), 28);
}

#[test]
fn a_reference_to_a_node_or_a_wire_is_refused_in_a_cell_and_an_attribute() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (template, code) in REFUSALS {
        for (reference, attribute, _) in REFERENCES {
            let text = format!(
                "{TARGETS}{}",
                template.replace('R', reference).replace('A', attribute)
            );
            let found: Vec<Code> = errors(&compiler, &text)
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect();
            if found.first() != Some(&code) {
                failures.push(format!("{text}\n  draws {found:?}, not {code:?} first\n"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_paths_of_the_spec_read_through_a_let_holding_the_wire_and_the_node() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let text = "Clip verse | gain=0.8:\n    Note as hook /65 | velocity=0.9\n\nClip chorus | gain=verse.gain * 1.2:\n    Note/72 | velocity=verse.hook.velocity\n    Plays as back | transpose=-2: verse\n\nlet hook = verse.hook\nlet back = chorus.back\n\nClip outro | gain=back.transpose:\n    Note/60 | velocity=hook.velocity\n";
    assert_eq!(errors(&compiler, text), vec![], "{text}");
}
