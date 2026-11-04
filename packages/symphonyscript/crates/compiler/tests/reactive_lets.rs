mod support;

use support::Compiler;
use symphonyscript::Code;
use symphonyscript_fixtures::vocabulary;

const CELLS: &str = "state note(60)\nstate flag(false)\nstate held([]): i32[..4]\nstate w(Wave::Saw)\nstate level(0.5)\nClip verse | gain=0.5:\n    Note\nClip bound | gain=level:\n    Note\n";

const READS: [(&str, &[&str]); 12] = [
    ("note + 1", &[]),
    ("note.past(1, 0)", &[]),
    ("if note.stepped then 1 else 2", &[]),
    ("g() + 1", &["fn g(): note"]),
    ("if true then note else 1", &[]),
    ("match 1:\n    1: note\n    else: 2", &[]),
    ("(for x in held into s from 0: s + x)", &[]),
    ("held.size", &[]),
    ("i32(bound.gain)", &[]),
    ("[1, 2][note]", &[]),
    ("if flag then 1 else 2", &[]),
    ("match w:\n    Saw: 1\n    else: 2", &[]),
];

const VALUES: [(&str, &[&str]); 5] = [
    ("if false then note else 1", &[]),
    ("match 2:\n    1: note\n    else: 2", &[]),
    ("held.capacity", &[]),
    ("verse.gain", &[]),
    ("r(2)", &["fn r(n): if n < 1 then 0 else r(n - 1)"]),
];

fn holder(pattern: bool, value: &str, extra: &[&str]) -> Vec<String> {
    let mut items: Vec<String> = extra.iter().map(|item| format!("{item}\n")).collect();
    if pattern {
        items.push(format!(
            "pattern P():\n    let a = {}\n    Note/(i32(a))\nClip c:\n    P\n",
            value.replace('\n', "\n    ")
        ));
    } else {
        items.push(format!("let a = {value}\nClip c:\n    Note/(i32(a))\n"));
    }
    items
}

fn draws(compiler: &Compiler, items: &[String], reversed: bool) -> (String, Vec<(Code, String)>) {
    let mut ordered: Vec<&str> = vec![CELLS];
    ordered.extend(items.iter().map(String::as_str));
    if reversed {
        ordered.reverse();
    }
    let text = ordered.concat();
    let found = support::error_texts(compiler, &text);
    (text, found)
}

#[test]
fn a_top_level_or_pattern_let_that_reads_a_cell_draws_reactive_let_at_its_first_reader_433() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for pattern in [false, true] {
        for reversed in [false, true] {
            for (value, extra) in READS {
                let (text, found) = draws(&compiler, &holder(pattern, value, extra), reversed);
                if found != [(Code::ReactiveLet, "a".to_string())] {
                    failures.push(support::failed_row_with(
                        &compiler,
                        &text,
                        format!("{text}found {found:?}\n"),
                    ));
                }
                let mut chained = extra
                    .iter()
                    .map(|item| format!("{item}\n"))
                    .collect::<Vec<_>>();
                chained.push(format!("let b = {value}\n"));
                chained.extend(holder(pattern, "b + 0", &[]));
                let (text, found) = draws(&compiler, &chained, reversed);
                if found != [(Code::ReactiveLet, "b".to_string())] {
                    failures.push(support::failed_row_with(
                        &compiler,
                        &text,
                        format!("{text}found {found:?}\n"),
                    ));
                }
                let mut state = extra
                    .iter()
                    .map(|item| format!("{item}\n"))
                    .collect::<Vec<_>>();
                state.push(format!("state b = {value}\n"));
                state.extend(holder(false, "b", &[]));
                let (text, found) = draws(&compiler, &state, reversed);
                if found != [(Code::ReactiveLet, "a".to_string())] {
                    failures.push(support::failed_row_with(
                        &compiler,
                        &text,
                        format!("{text}found {found:?}\n"),
                    ));
                }
            }
            for (value, extra) in VALUES {
                let (text, found) = draws(&compiler, &holder(pattern, value, extra), reversed);
                if !found.is_empty() {
                    failures.push(support::failed_row_with(
                        &compiler,
                        &text,
                        format!("{text}found {found:?}\n"),
                    ));
                }
            }
        }
    }
    support::assert_no_failure(&failures);
}

#[test]
fn a_let_holding_a_lambda_or_a_functions_local_let_reading_a_cell_draws_no_reactive_let_433() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for reversed in [false, true] {
        let items = vec!["let f = y: note + y\nClip c:\n    Note\n".to_string()];
        let (text, found) = draws(&compiler, &items, reversed);
        let codes: Vec<Code> = found.iter().map(|(code, _)| *code).collect();
        assert_eq!(codes, [Code::UnbuiltConstruct], "{text}");
        let items =
            vec!["fn h(k):\n    let d = k + 1\n    d\nClip c:\n    Note/(h(note))\n".to_string()];
        let (text, found) = draws(&compiler, &items, reversed);
        assert_eq!(found, [], "{text}");
    }
}
