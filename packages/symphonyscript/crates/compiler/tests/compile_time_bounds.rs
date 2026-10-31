mod support;

use std::collections::BTreeSet;

use symphonyscript::{Code, Compiler, Source};
use symphonyscript_fixtures::{Rows, covered, native_vocabulary};

#[derive(Clone)]
struct Base {
    declarations: &'static [&'static str],
    value: &'static str,
    history: bool,
    length_known: bool,
}

fn bases() -> Vec<(Base, String)> {
    let mut all: Vec<(Base, String)> = BASES
        .iter()
        .map(|base| (base.clone(), base.value.to_string()))
        .collect();
    for base in &BASES {
        all.push((base.clone(), format!("({})[0..]", base.value)));
    }
    all
}

const LITERAL: Base = Base {
    declarations: &[],
    value: "[1, 2, 3]",
    history: false,
    length_known: true,
};

const BASES: [Base; 13] = [
    LITERAL,
    Base {
        declarations: &["let a = [1, 2, 3]\n"],
        value: "a",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state xf([1, 2, 3])\n"],
        value: "xf",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n"],
        value: "xs",
        history: false,
        length_known: false,
    },
    Base {
        declarations: &["state xf([1, 2, 3])\n"],
        value: "xf.past(1)",
        history: true,
        length_known: true,
    },
    Base {
        declarations: &["state xf([1, 2, 3])\n"],
        value: "xf.past(1, [0, 0, 0])",
        history: true,
        length_known: true,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n"],
        value: "xs.past(1)",
        history: true,
        length_known: false,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n"],
        value: "xs.past(1, [0])",
        history: true,
        length_known: false,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n", "state s = xs\n"],
        value: "s",
        history: false,
        length_known: false,
    },
    Base {
        declarations: &[],
        value: "\"abc\".bytes",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state xf([1, 2, 3])\n"],
        value: "xf.bytes",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state n(1)\n"],
        value: "three(n)",
        history: false,
        length_known: true,
    },
    Base {
        declarations: &["state xs([1, 2, 3]): i32[..5]\n"],
        value: "mirror(xs)",
        history: false,
        length_known: false,
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Position {
    Index,
    Lower,
    Upper,
    IndexFromEnd,
    LowerFromEnd,
    UpperFromEnd,
}

const POSITIONS: [Position; 6] = [
    Position::Index,
    Position::Lower,
    Position::Upper,
    Position::IndexFromEnd,
    Position::LowerFromEnd,
    Position::UpperFromEnd,
];

const OUTSIDE: [&str; 4] = [
    "2 ** 63",
    "-(2 ** 63) - 1",
    "2 ** 70",
    "-170141183460469231731687303715884105727",
];

const INSIDE: [&str; 4] = ["2 ** 63 - 1", "-(2 ** 63)", "0", "-1"];

impl Position {
    fn bracket(self, value: &str) -> (String, usize) {
        let (before, after) = match self {
            Position::Index => ("[", "]"),
            Position::Lower => ("[", "..]"),
            Position::Upper => ("[..", "]"),
            Position::IndexFromEnd => ("[^", "]"),
            Position::LowerFromEnd => ("[^", "..]"),
            Position::UpperFromEnd => ("[..^", "]"),
        };
        (format!("{before}{value}{after}"), before.len())
    }

    fn reads_one_element(self) -> bool {
        matches!(self, Position::Index | Position::IndexFromEnd)
    }
}

struct Built {
    text: String,
    bound: usize,
}

fn build(
    base: &str,
    declarations: &[&str],
    history: bool,
    position: Position,
    value: &str,
    reversed: bool,
) -> Built {
    let (bracket, offset) = position.bracket(value);
    let indexed = format!("{base}{bracket}");
    let read = if position.reads_one_element() {
        indexed
    } else {
        format!("({indexed}).size")
    };
    let mut items: Vec<String> = declarations.iter().map(|item| item.to_string()).collect();
    let (state, pitch) = if history {
        (Some(format!("state h = {read}\n")), "h".to_string())
    } else {
        (None, read)
    };
    items.extend(state);
    items.push(format!("Song g:\n    Clip:\n        Note/({pitch})\n"));
    if reversed {
        items.reverse();
    }
    let text = items.concat();
    let bracketed = format!("{base}{bracket}");
    let start = text.find(&bracketed).expect("the program holds its bound");
    Built {
        bound: start + base.len() + offset,
        text,
    }
}

type Found = BTreeSet<(Code, isize, isize, bool)>;

fn found(compiler: &Compiler, built: &Built) -> Found {
    let bound = isize::try_from(built.bound).unwrap_or(isize::MAX);
    support::errors(compiler.check(&[Source {
        path: "main.sym".into(),
        text: built.text.clone(),
    }]))
    .into_iter()
    .map(|diagnostic| {
        let at = |offset: u32| isize::try_from(offset).unwrap_or(isize::MAX) - bound;
        (
            diagnostic.code,
            at(diagnostic.span.range.start).max(-1),
            at(diagnostic.span.range.end),
            diagnostic.summary.contains("outside `i64`"),
        )
    })
    .collect()
}

fn outside_i64_failures(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&native_vocabulary()).expect("the native vocabulary compiles");
    let bases = bases();
    let mut failures = Vec::new();
    let mut programs = 0;
    for (position_at, position) in POSITIONS.into_iter().enumerate() {
        for (value_at, (outside, value)) in OUTSIDE
            .iter()
            .map(|value| (true, *value))
            .chain(INSIDE.iter().map(|value| (false, *value)))
            .enumerate()
        {
            for reversed in [false, true] {
                let taken: Vec<usize> = (0..bases.len())
                    .filter(|base_at| {
                        rows.take(&[position_at, value_at, usize::from(reversed), *base_at])
                    })
                    .collect();
                if taken.is_empty() {
                    continue;
                }
                let twin = build(
                    LITERAL.value,
                    LITERAL.declarations,
                    false,
                    position,
                    value,
                    reversed,
                );
                let twin_found = found(&compiler, &twin);
                let length = isize::try_from(value.len()).unwrap_or(isize::MAX);
                if outside {
                    let at_bound = Found::from([(Code::IndexOutOfRange, 0, length, true)]);
                    let plain_index = Found::from([(Code::IndexOutOfRange, 0, length, false)]);
                    let expected = if position == Position::Index {
                        &plain_index
                    } else {
                        &at_bound
                    };
                    if twin_found != *expected {
                        failures.push(format!("{}\n  twin draws {twin_found:?}\n", twin.text));
                    }
                }
                for base_at in taken {
                    let (base, written) = &bases[base_at];
                    let built = build(
                        written,
                        base.declarations,
                        base.history,
                        position,
                        value,
                        reversed,
                    );
                    let found = found(&compiler, &built);
                    programs += 1;
                    let expected: Found = if outside && base.length_known {
                        twin_found.clone()
                    } else if outside {
                        twin_found
                            .iter()
                            .map(|(code, start, end, _)| (*code, *start, *end, true))
                            .collect()
                    } else if base.length_known {
                        twin_found.clone()
                    } else {
                        twin_found
                            .iter()
                            .filter(|(code, ..)| *code != Code::IndexOutOfRange)
                            .copied()
                            .collect()
                    };
                    if found != expected {
                        failures.push(format!(
                            "{}\n  found {found:?}\n  expected {expected:?}\n",
                            built.text
                        ));
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn assert_outside_i64(failures: &[String], programs: usize, expected: usize) {
    assert!(
        failures.is_empty(),
        "{} of {programs} programs fail:\n{}",
        failures.len(),
        failures.iter().take(12).cloned().collect::<String>()
    );
    assert_eq!(programs, expected);
}

#[test]
fn a_compile_time_index_bound_or_offset_outside_i64_draws_at_the_bound_whatever_the_base() {
    let (failures, programs) = covered(outside_i64_failures);
    assert_outside_i64(&failures, programs, 208);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_compile_time_index_bound_or_offset_outside_i64_draws_at_the_bound_whatever_the_base_400()
 {
    let (failures, programs) = outside_i64_failures(&Rows::Every);
    assert_outside_i64(&failures, programs, 2_496);
}

const FAULTY_BOUNDS: [(&str, Code, &str); 6] = [
    ("(1 + true)", Code::OperandType, "1 + true"),
    ("nothing", Code::UnknownName, "nothing"),
    ("1.5", Code::IndexType, "1.5"),
    ("[1, 2][5]", Code::IndexOutOfRange, "5"),
    ("i32(true)", Code::ConversionType, "i32(true)"),
    ("2 ** 63", Code::IndexOutOfRange, "2 ** 63"),
];

const SLICE_SIDES: [(&str, &str); 3] =
    [("xs[{b}..]", "3"), ("xs[..{b}]", "0"), ("xs[^{b}..]", "0")];

const SLICE_READS: [&str; 2] = ["[0]", ".size"];

const SLICE_PLACES: [[&str; 2]; 3] = [
    ["let xs = [1, 2, 3]\n", "Clip c:\n    Note/({read})\n"],
    ["fn f(xs): {read}\n", "Clip c:\n    Note/(f([1, 2, 3]))\n"],
    [
        "fn r(xs, n): if n > 0 then r(xs, n - 1) else {read}\n",
        "Clip c:\n    Note/(r([1, 2, 3], 2))\n",
    ],
];

fn slice_bound_rows(rows: &symphonyscript_fixtures::Rows) -> Vec<String> {
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut check = |items: &[String], expected: Vec<(Code, String)>| {
        for order in [false, true] {
            let mut ordered = items.to_vec();
            if order {
                ordered.reverse();
            }
            let text = ordered.concat();
            let found = support::error_texts(&compiler, &text);
            if found != expected {
                failures.push(format!(
                    "356: {text}\n  draws {found:?}\n  expected {expected:?}"
                ));
            }
        }
    };
    for (fault, (bound, code, spanned)) in FAULTY_BOUNDS.iter().enumerate() {
        for (side, (form, twin)) in SLICE_SIDES.iter().enumerate() {
            for (read, suffix) in SLICE_READS.iter().enumerate() {
                for (place, items) in SLICE_PLACES.iter().enumerate() {
                    if !rows.take(&[fault, side, read, place]) {
                        continue;
                    }
                    let built = |value: &str| -> Vec<String> {
                        let read = format!("{}{suffix}", form.replace("{b}", value));
                        items
                            .iter()
                            .map(|item| item.replace("{read}", &read))
                            .collect()
                    };
                    check(&built(bound), vec![(*code, (*spanned).to_string())]);
                    let drawn = if read == 0 {
                        vec![(Code::IndexOutOfRange, "0".to_string())]
                    } else {
                        Vec::new()
                    };
                    check(&built(twin), drawn);
                }
            }
        }
    }
    failures
}

#[test]
fn a_slice_whose_bound_is_in_error_reads_no_value_so_only_the_bound_draws() {
    let failures = symphonyscript_fixtures::covered(slice_bound_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_slice_whose_bound_is_in_error_reads_no_value_so_only_the_bound_draws() {
    let failures = slice_bound_rows(&symphonyscript_fixtures::Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const PLACEHOLDER_BASES: [(&[&str], &str, &str); 4] = [
    (&[], "[1, true]", "[1, true]"),
    (
        &[],
        "(if 1 > 0 then [1] else [true])",
        "if 1 > 0 then [1] else [true]",
    ),
    (&["let a = [1, true]\n"], "a", "[1, true]"),
    (&[], "([1, true] + [2])", "[1, true]"),
];

const BOUND_FAULTS: [(&str, Option<(Code, &str)>); 5] = [
    ("2 ** 63", Some((Code::IndexOutOfRange, "2 ** 63"))),
    ("1.5", Some((Code::IndexType, "1.5"))),
    ("true", Some((Code::IndexType, "true"))),
    ("(1 + true)", Some((Code::OperandType, "1 + true"))),
    ("0", None),
];

const BOUND_FORMS: [&str; 4] = [
    "{base}[{b}..]",
    "{base}[..{b}]",
    "{base}[^{b}..]",
    "{base}[{b}]",
];

fn placeholder_base_rows(rows: &symphonyscript_fixtures::Rows) -> Vec<String> {
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (base, (declarations, value, faulty)) in PLACEHOLDER_BASES.iter().enumerate() {
        for (fault, (bound, drawn)) in BOUND_FAULTS.iter().enumerate() {
            for (form, shape) in BOUND_FORMS.iter().enumerate() {
                for (read, suffix) in ["", ".size"].iter().enumerate() {
                    if form == 3 && read == 1
                        || drawn.is_none() && form < 3 && read == 0
                        || base == 3 && drawn.is_some()
                    {
                        continue;
                    }
                    let suffix = if form == 3 || read == 1 {
                        *suffix
                    } else {
                        "[0]"
                    };
                    let read_text = format!(
                        "{}{suffix}",
                        shape.replace("{base}", value).replace("{b}", bound)
                    );
                    let mut items: Vec<String> = declarations
                        .iter()
                        .map(|item| (*item).to_string())
                        .collect();
                    items.push(format!("Clip c:\n    Note/({read_text})\n"));
                    let mut expected = vec![(Code::TypeMismatch, (*faulty).to_string())];
                    if let Some((code, spanned)) = drawn {
                        expected.push((*code, (*spanned).to_string()));
                    }
                    expected.sort();
                    for (order, reversed) in [false, true].into_iter().enumerate() {
                        if !rows.take(&[base, fault, form, read, order]) {
                            continue;
                        }
                        let mut ordered = items.clone();
                        if reversed {
                            ordered.reverse();
                        }
                        let text = ordered.concat();
                        let found = support::error_texts(&compiler, &text);
                        if found != expected {
                            failures.push(format!(
                                "311: {text}\n  draws {found:?}\n  expected {expected:?}"
                            ));
                        }
                    }
                }
            }
        }
    }
    failures
}

#[test]
fn a_bound_beside_a_placeholder_base_draws_its_own_fault_and_the_base_draws_once() {
    let failures = symphonyscript_fixtures::covered(placeholder_base_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_bound_beside_a_placeholder_base_draws_its_own_fault_and_the_base_draws_once() {
    let failures = placeholder_base_rows(&symphonyscript_fixtures::Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const UNREAD_FAULTS: [(&str, Code, bool); 2] = [
    ("[1, 2][5]", Code::IndexOutOfRange, false),
    ("2147483647 + 1", Code::DoesNotFit, true),
];

const SELECTING_READS: [&str; 6] = [
    "{a}[{p}]",
    "{a}[^{q}]",
    "{a}[{a}.size - {q}]",
    "{a}[i]",
    "{a}[{p}..][0]",
    "{a}[^{q}..][0]",
];

fn unread_element_rows(rows: &symphonyscript_fixtures::Rows) -> Vec<String> {
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (fault, (element, code, at_read)) in UNREAD_FAULTS.iter().enumerate() {
        for faulty_at in 0..2usize {
            let literal = if faulty_at == 0 {
                format!("[{element}, 3]")
            } else {
                format!("[3, {element}]")
            };
            for (named, base) in [&literal[..], "a"].into_iter().enumerate() {
                if *at_read && named == 1 {
                    continue;
                }
                for (form, shape) in SELECTING_READS.iter().enumerate() {
                    for selected in 0..2usize {
                        if !rows.take(&[fault, faulty_at, named, form, selected]) {
                            continue;
                        }
                        let read = shape
                            .replace("{a}", base)
                            .replace("{p}", &selected.to_string())
                            .replace("{q}", &(2 - selected).to_string());
                        let mut items = vec![
                            format!("let i = {selected}\n"),
                            format!("Clip c:\n    Note/({read})\n"),
                        ];
                        if named == 1 {
                            items.push(format!("let a = {literal}\n"));
                        }
                        let expected = if selected != faulty_at {
                            Vec::new()
                        } else if *at_read {
                            vec![(*code, read.clone())]
                        } else {
                            vec![(*code, "5".to_string())]
                        };
                        for reversed in [false, true] {
                            let mut ordered = items.clone();
                            if reversed {
                                ordered.reverse();
                            }
                            let text = ordered.concat();
                            let found = support::error_texts(&compiler, &text);
                            if found != expected {
                                failures.push(format!(
                                    "290: {text}\n  draws {found:?}\n  expected {expected:?}"
                                ));
                            }
                        }
                    }
                }
                let text = format!(
                    "Clip c:\n    Note/({base}.size)\n{}",
                    if named == 1 {
                        format!("let a = {literal}\n")
                    } else {
                        String::new()
                    }
                );
                let found = support::error_texts(&compiler, &text);
                if !found.is_empty() {
                    failures.push(format!("290: {text}\n  draws {found:?}"));
                }
            }
        }
    }
    failures
}

#[test]
fn every_compile_time_index_form_reads_only_the_element_it_selects() {
    let failures = symphonyscript_fixtures::covered(unread_element_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_every_compile_time_index_form_reads_only_the_element_it_selects() {
    let failures = unread_element_rows(&symphonyscript_fixtures::Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_runtime_index_reads_every_element_and_draws_its_fault() {
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let text = "state k(1)\nClip c:\n    Note/([[1, 2][5], 3][k])\n";
    assert_eq!(
        support::error_texts(&compiler, text),
        vec![(Code::IndexOutOfRange, "5".to_string())],
        "290: {text}"
    );
}

#[test]
fn an_offset_from_the_end_of_a_runtime_base_is_read_in_its_own_type_as_its_literal_twin_reads_it_318()
 {
    let held = "state xs([1, 2, 3]): i32[..5]\n";
    let rows: &[support::Row] = &[
        (&[held, "Clip c:\n    Note/(xs[^2 ** 63 - 1])\n"], &[]),
        (&[held, "Clip c:\n    Note/(xs[^-(2 ** 63)])\n"], &[]),
        (&[held, "Clip c:\n    Note/(xs[^300000000000])\n"], &[]),
        (&[held, "Clip c:\n    Note/(xs[^2 ** 63 - 1..][0])\n"], &[]),
        (
            &["Clip c:\n    Note/([1, 2, 3][^2 ** 63 - 1])\n"],
            &[(Code::IndexOutOfRange, "[1, 2, 3][^2 ** 63 - 1]")],
        ),
        (
            &[held, "Clip c:\n    Note/(xs[^2 ** 63])\n"],
            &[(Code::IndexOutOfRange, "2 ** 63")],
        ),
        (
            &[held, "Clip c:\n    Note/(xs[^1.5])\n"],
            &[(Code::IndexType, "xs[^1.5]")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn an_index_an_offset_from_the_end_or_a_slice_of_a_single_value_draws_not_an_array_once_399() {
    let n = "let n = 5\n";
    let s = "state s0 = 5\n";
    let rows: &[support::Row] = &[
        (
            &[n, "Clip c:\n    Note/(n[^1.5])\n"],
            &[(Code::NotAnArray, "n")],
        ),
        (
            &[n, "Clip c:\n    Note/(n[^1])\n"],
            &[(Code::NotAnArray, "n")],
        ),
        (
            &[s, "Clip c:\n    Note/(s0[^1])\n"],
            &[(Code::NotAnArray, "s0")],
        ),
        (
            &[n, "Clip c:\n    Note/(n[0])\n"],
            &[(Code::NotAnArray, "n")],
        ),
        (
            &[s, "Clip c:\n    Note/(s0[0])\n"],
            &[(Code::NotAnArray, "s0")],
        ),
        (
            &[n, "Clip c:\n    Note/(n[0..1][0])\n"],
            &[(Code::NotAnArray, "n")],
        ),
        (
            &[n, "Clip c:\n    Note/(n[^1..][0])\n"],
            &[(Code::NotAnArray, "n")],
        ),
        (
            &[n, "Clip c:\n    Note/(n[^[1, 2][5]])\n"],
            &[(Code::NotAnArray, "n"), (Code::IndexOutOfRange, "5")],
        ),
        (
            &[n, "Clip c:\n    Note/(\"abc\"[^n])\n"],
            &[(Code::NotAnArray, "\"abc\"")],
        ),
        (&[n, "Clip c:\n    Note/([1, 2][^n - 3])\n"], &[]),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}
