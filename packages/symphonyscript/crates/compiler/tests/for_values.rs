mod support;

use symphonyscript::{Code, Compiler, Diagnostic, Severity, Source};
use symphonyscript_core::LimitOverrides;
use symphonyscript_fixtures::{Rows, covered, vocabulary};

const CELLS: &str = "state held([]): i32[..10]\nstate a(3)\nstate b(5)\nstate c(7)\nstate n(2)\n";

type Check = Box<dyn Fn(&[Diagnostic]) -> bool>;

enum Folded {
    Value(Vec<i128>),
    Sum(i128),
    Cell,
    Capacity(u32),
}

struct Iterable {
    written: &'static str,
    values: Option<&'static [i128]>,
    count: u32,
}

const ITERABLES: [Iterable; 5] = [
    Iterable {
        written: "[3, 5, 7]",
        values: Some(&[3, 5, 7]),
        count: 3,
    },
    Iterable {
        written: "1..4",
        values: Some(&[1, 2, 3]),
        count: 3,
    },
    Iterable {
        written: "held",
        values: None,
        count: 10,
    },
    Iterable {
        written: "[a, b, c]",
        values: None,
        count: 3,
    },
    Iterable {
        written: "0..min(n, 3)",
        values: None,
        count: 3,
    },
];

const BODIES: [(&str, &str); 5] = [
    ("0", "s + x"),
    ("[]", "s"),
    ("[]", "s + [x]"),
    ("[]", "(s + [x])[^2..]"),
    ("0", "s + (for y in [x, x] into t from 0: t + y)"),
];

const POSITIONS: [&str; 3] = ["V", "fn f(p): F\nV", "V"];

fn folded(iterable: &Iterable, body: usize) -> Folded {
    let Some(values) = iterable.values else {
        return match body {
            0 | 4 => Folded::Cell,
            1 => Folded::Capacity(0),
            2 => Folded::Capacity(iterable.count),
            _ => Folded::Capacity(iterable.count.min(2)),
        };
    };
    match body {
        0 => Folded::Sum(values.iter().sum()),
        4 => Folded::Sum(2 * values.iter().sum::<i128>()),
        1 => Folded::Value(Vec::new()),
        2 => Folded::Value(values.to_vec()),
        _ => Folded::Value(
            values
                .iter()
                .skip(values.len().saturating_sub(2))
                .copied()
                .collect(),
        ),
    }
}

fn program(iterable: &Iterable, body: usize, position: usize, value: &str) -> String {
    let word = if iterable.values.is_some() {
        "let"
    } else {
        "state"
    };
    let start = BODIES[body].0;
    let read = match position {
        1 => "f(0)".to_string(),
        2 if start == "0" => format!("0 + ({value})"),
        2 => format!("[] + ({value})"),
        _ => value.to_string(),
    };
    let declaration = POSITIONS[position]
        .replace("F", value)
        .replace('V', &format!("{word} v = {read}"));
    format!("{CELLS}{declaration}\n")
}

fn errors(compiler: &Compiler, text: &str) -> Vec<Diagnostic> {
    support::errors(compiler.check(&[support::source(text)]))
}

fn outside(found: &[Diagnostic], indexes: &[i128]) -> bool {
    found.len() == indexes.len()
        && found.iter().zip(indexes).all(|(diagnostic, index)| {
            diagnostic.code == Code::IndexOutOfRange
                && diagnostic
                    .summary
                    .starts_with(&format!("the index {index} is outside"))
        })
}

fn holds(compiler: &Compiler, text: &str, folded: &Folded) -> Result<(), String> {
    let (reader, ok): (String, Check) = match folded {
        Folded::Sum(sum) => {
            let sum = *sum;
            (
                "Clip k:\n    Note/([0][v])\n".to_string(),
                Box::new(move |found: &[Diagnostic]| outside(found, &[sum])),
            )
        }
        Folded::Value(values) => {
            let length = i128::try_from(values.len()).unwrap_or(0);
            let mut indexes = vec![length + 1];
            let mut reader = "Clip k:\n    Note/([0][v.size + 1])\n".to_string();
            if let Some(last) = values.last() {
                indexes.push(last + 1);
                reader.push_str("    Note/([0][v[v.size - 1] + 1])\n");
            }
            (
                reader,
                Box::new(move |found: &[Diagnostic]| outside(found, &indexes)),
            )
        }
        Folded::Cell => (
            "Clip k:\n    Note/(v)\n".to_string(),
            Box::new(|found: &[Diagnostic]| found.is_empty()),
        ),
        Folded::Capacity(capacity) => {
            let kept = (*capacity).max(1);
            let checked =
                format!("{text}state probe: i32[..{kept}] = v\nClip k:\n    Note/(probe[0])\n");
            let found = errors(compiler, &checked);
            if !found.is_empty() {
                return Err(format!("{checked}\n  draws {found:?}\n"));
            }
            if *capacity < 2 {
                return Ok(());
            }
            let below = capacity - 1;
            let checked =
                format!("{text}state probe: i32[..{below}] = v\nClip k:\n    Note/(probe[0])\n");
            let found = errors(compiler, &checked);
            return match found.as_slice() {
                [diagnostic] if diagnostic.code == Code::CapacityExceeded => Ok(()),
                _ => Err(format!(
                    "{checked}\n  draws {found:?}, not capacity-exceeded\n"
                )),
            };
        }
    };
    let checked = format!("{text}{reader}");
    let found = errors(compiler, &checked);
    if ok(&found) {
        Ok(())
    } else {
        Err(format!("{checked}\n  draws {found:?}\n"))
    }
}

fn twin_value(iterable: &Iterable, folded: &Folded) -> String {
    match folded {
        Folded::Sum(sum) => sum.to_string(),
        Folded::Value(values) => format!(
            "[{}]",
            values
                .iter()
                .map(i128::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Folded::Cell => "held[0]".to_string(),
        Folded::Capacity(0) => "[]".to_string(),
        Folded::Capacity(capacity) if iterable.written == "held" && *capacity == 10 => {
            "held".to_string()
        }
        Folded::Capacity(capacity) => format!("held[..{capacity}]"),
    }
}

fn fold_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (at, iterable) in ITERABLES.iter().enumerate() {
        for (body, (start, step)) in BODIES.iter().enumerate() {
            for position in 0..POSITIONS.len() {
                if !rows.take(&[at, body, position]) {
                    continue;
                }
                programs += 1;
                let expected = folded(iterable, body);
                let value = format!("for x in {} into s from {start}: {step}", iterable.written);
                let text = program(iterable, body, position, &value);
                let twin = program(iterable, body, position, &twin_value(iterable, &expected));
                let outcome = holds(&compiler, &twin, &expected)
                    .map_err(|failure| format!("the twin fails: {failure}"))
                    .and_then(|()| holds(&compiler, &text, &expected));
                failures.extend(
                    outcome
                        .err()
                        .map(|failure| support::failed_row_with(&compiler, &text, failure)),
                );
            }
        }
    }
    (failures, programs)
}

#[test]
fn a_for_expression_folds_its_iterable_into_the_value_or_the_capacity_its_rules_give() {
    support::assert_none_fail(covered(fold_rows), 25);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_for_expression_folds_its_iterable_into_the_value_or_the_capacity_its_rules_give() {
    support::assert_none_fail(fold_rows(&Rows::Every), 75);
}

#[test]
fn the_for_expressions_of_the_spec_hold_the_values_and_capacities_the_spec_gives() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let held = "state held([]): i32[..10]\n";
    let high = "state high = for x in held into kept from []:\n    let keep = x > 60\n    if keep then kept + [x] else kept\n";
    let rises = "state rises = for i in 1..held.size into n from 0: if held[i] > held[i - 1] then n + 1 else n\n";
    let lowest = "state lowest = for x in held into low from 127: min(low, x)\n";
    for (text, expected) in [
        (
            format!("{held}{high}state probe: i32[..10] = high\nClip c:\n    Note/(probe[0])\n"),
            vec![],
        ),
        (
            format!("{held}{high}state probe: i32[..9] = high\nClip c:\n    Note/(probe[0])\n"),
            vec![Code::CapacityExceeded],
        ),
        (
            format!("{held}{rises}{lowest}Clip c:\n    Note/(rises + lowest)\n"),
            vec![],
        ),
        (
            format!(
                "{held}state total = 1 + (for i, x in held into sum from 0: sum + i * x)\nClip c:\n    Note/total\n"
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
    let counted = |ty: &str| {
        format!(
            "{held}state r = for i in 1..held.size into n: {ty} from 0: n + 1\nClip c:\n    Note/(r)\n"
        )
    };
    let reached = format!(
        "{held}state rises = for i in 1..held.size into n from []: n + [i]\nstate probe: i32[..8] = rises\nClip c:\n    Note/(probe[0])\n"
    );
    let found: Vec<Code> = errors(&compiler, &reached)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(found, [Code::CapacityExceeded], "{reached}");
    let kept = reached.replace("i32[..8]", "i32[..9]");
    assert_eq!(errors(&compiler, &kept), vec![], "{kept}");
    assert_eq!(
        errors(&compiler, &counted("i32")),
        vec![],
        "the annotated count"
    );
}

#[test]
fn a_compile_time_for_counts_each_iteration_toward_the_expansion_limit_and_its_warning() {
    let compiler = Compiler::with_limits(
        &vocabulary(),
        LimitOverrides {
            expansion_steps: Some(2_000),
            expansion_steps_warning: Some(1_000),
            ..LimitOverrides::default()
        },
    )
    .expect("the example vocabulary compiles");
    for (count, expected) in [
        (1_000, vec![]),
        (1_001, vec![(Severity::Warning, Code::LimitWarning)]),
        (
            4_000,
            vec![
                (Severity::Error, Code::ExpansionLimit),
                (Severity::Warning, Code::LimitWarning),
            ],
        ),
    ] {
        let text = format!(
            "let v = for i in 0..{count} into s from 0: s + i\nClip c:\n    Note/(v)\nSession main\n"
        );
        let found: Vec<(Severity, Code)> = compiler
            .check(&[Source {
                path: "count.sym".into(),
                text: text.clone(),
            }])
            .iter()
            .filter(|diagnostic| diagnostic.code != Code::UnreachableInstance)
            .map(|diagnostic| (diagnostic.severity(), diagnostic.code))
            .collect();
        assert_eq!(found, expected, "{text}");
    }
}

#[test]
fn a_for_expression_starts_from_its_start_and_yields_the_start_over_an_empty_iterable() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (value, folded) in [
        ("for x in [3, 5, 7] into s from 10: s + x", Folded::Sum(25)),
        (
            "for x in [3, 5] into s from [9]: s + [x]",
            Folded::Value(vec![9, 3, 5]),
        ),
        ("for x in [] into s from 4: s + x", Folded::Sum(4)),
        (
            "for x in [] into s from [7, 8]: s + [x]",
            Folded::Value(vec![7, 8]),
        ),
        ("for i in 4..4 into s from 9: s + i", Folded::Sum(9)),
        ("for i in 5..2 into s from 9: s + i", Folded::Sum(9)),
        (
            "for i in 5..2 into s from [6]: s + [i]",
            Folded::Value(vec![6]),
        ),
    ] {
        let text = format!("{CELLS}let v = {value}\n");
        failures.extend(
            holds(&compiler, &text, &folded)
                .err()
                .map(|failure| support::failed_row_with(&compiler, &text, failure)),
        );
    }
    support::assert_no_failure(&failures);
}

const READERS: [(&str, &str, &str, &str, &str); 4] = [
    ("[3, 5, 7]", "3..6", "0", "s + x", ""),
    ("[3, 5, c]", "3..min(c, 6)", "0", "s + x", "the iterable"),
    ("[3, 5, 7]", "3..6", "n", "s + x", "the start"),
    ("[3, 5, 7]", "3..6", "0", "s + x + n", "the body"),
];

const PLACES: [&str; 3] = [
    "let v = F\n",
    "fn f(p): F\nlet v = f(0)\n",
    "let v = 0 + (F)\n",
];

fn cell_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (reader, (array, range, start, body, _)) in READERS.iter().enumerate() {
        for (form, (iterable, sum)) in [(array, 15), (range, 12)].into_iter().enumerate() {
            for (place, template) in PLACES.iter().enumerate() {
                if !rows.take(&[reader, form, place]) {
                    continue;
                }
                programs += 1;
                let value = format!("for x in {iterable} into s from {start}: {body}");
                let text = format!("{CELLS}{}", template.replace('F', &value));
                let expected = if reader == 0 {
                    Folded::Sum(sum)
                } else {
                    Folded::Cell
                };
                let read = format!("{text}Clip k:\n    Note/([0][v])\n");
                let outcome = match expected {
                    Folded::Cell => {
                        let found = errors(&compiler, &read);
                        let at = read.find("let v").map(|at| at + "let ".len());
                        match found.as_slice() {
                            [diagnostic]
                                if diagnostic.code == Code::ReactiveLet
                                    && usize::try_from(diagnostic.span.range.start).ok() == at =>
                            {
                                Ok(())
                            }
                            _ => Err(format!(
                                "{read}\n  draws {found:?}, not reactive-let at `v` (433)\n"
                            )),
                        }
                    }
                    Folded::Sum(_) | Folded::Value(_) | Folded::Capacity(_) => {
                        holds(&compiler, &text, &expected)
                    }
                };
                failures.extend(
                    outcome
                        .err()
                        .map(|failure| support::failed_row_with(&compiler, &text, failure)),
                );
            }
        }
    }
    (failures, programs)
}

#[test]
fn a_for_expression_is_a_compile_time_value_only_when_its_iterable_start_and_body_read_no_cell() {
    support::assert_none_fail(covered(cell_rows), 12);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_for_expression_is_a_compile_time_value_only_when_its_iterable_start_and_body_read_no_cell()
 {
    support::assert_none_fail(cell_rows(&Rows::Every), 24);
}

#[test]
fn a_for_expressions_body_counts_once_toward_the_inline_limit_however_many_times_it_runs() {
    let compiler = Compiler::with_limits(
        &vocabulary(),
        LimitOverrides {
            inlined_calls: Some(10),
            ..LimitOverrides::default()
        },
    )
    .expect("the example vocabulary compiles");
    let calls = |count: usize| vec!["f(x)"; count].join(" + ");
    for (iterable, count, expected) in [
        ("0..100", 1, vec![]),
        ("0..100", 10, vec![]),
        ("0..2", 6, vec![]),
        ("0..2", 11, vec![Code::InlineLimit]),
        ("held", 10, vec![]),
        ("held", 11, vec![Code::InlineLimit]),
    ] {
        let word = if iterable == "held" { "state" } else { "let" };
        let text = format!(
            "{CELLS}fn f(y): y\n{word} v = for x in {iterable} into s from 0: s + {}\nClip k:\n    Note/(v)\n",
            calls(count)
        );
        let found: Vec<Code> = errors(&compiler, &text)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert_eq!(found, expected, "{text}");
    }
}

#[test]
fn the_accumulator_takes_the_type_its_start_and_body_unify_to_as_if_arms_do_and_an_annotation_fixes_it()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let reading = |value: &str| format!("let v = {value}\nClip k:\n    Note/([0, 0][v])\n");
    let at = |text: &str, diagnostic: &Diagnostic| {
        text.get(diagnostic.span.range.start as usize..diagnostic.span.range.end as usize)
            .unwrap_or_default()
            .to_string()
    };
    for (value, twin, code, spanned) in [
        (
            "for x in [1, 2] into s from 0: s + 0.5",
            Some("if true then 0 else 0.5"),
            Code::IndexType,
            "v",
        ),
        (
            "for x in [1, 2] into s from 0.5: s + x",
            Some("if true then 0.5 else 1"),
            Code::IndexType,
            "v",
        ),
        (
            "for x in [1, 2] into s from 0: s + x",
            None,
            Code::IndexOutOfRange,
            "v",
        ),
        (
            "for x in [1, 2] into s: f32 from 0: s + x",
            None,
            Code::IndexType,
            "v",
        ),
        (
            "for x in [1, 2] into s from []: 0",
            Some("if true then [] else 0"),
            Code::TypeMismatch,
            "",
        ),
        (
            "for x in [1, 2] into s from 0: [x]",
            Some("if true then 0 else [1]"),
            Code::TypeMismatch,
            "",
        ),
    ] {
        for written in std::iter::once(value).chain(twin) {
            let text = reading(written);
            let found = errors(&compiler, &text);
            let place = if spanned.is_empty() { written } else { spanned };
            let helped = |diagnostic: &Diagnostic| {
                code != Code::TypeMismatch
                    || !written.starts_with("for")
                    || diagnostic.help.as_deref()
                        == Some(
                            "give the start and the body values of one type and shape, as the two arms of an `if` are",
                        )
            };
            assert!(
                matches!(found.as_slice(), [diagnostic]
                    if diagnostic.code == code && at(&text, diagnostic) == place && helped(diagnostic)),
                "{text}\n  draws {found:?}, not one {code:?} at {place:?}"
            );
        }
    }
}

#[test]
fn a_capacity_past_an_annotated_accumulator_names_both_and_the_fix_its_help_names_passes() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let text = format!(
        "{CELLS}state t = for x in held into kept: i32[..3] from []: kept + [x]\nClip k:\n    Note/(t[0])\n"
    );
    let found = errors(&compiler, &text);
    assert!(
        matches!(found.as_slice(), [diagnostic]
            if diagnostic.code == Code::CapacityExceeded
                && diagnostic.summary.contains("10")
                && diagnostic.summary.contains("annotated capacity 3")
                && diagnostic.help.as_deref() == Some("slice the body's value to its newest 3 elements with `[^3..]`, or annotate a capacity of at least 10")),
        "{text}\n  draws {found:?}"
    );
    for fixed in [
        text.replace("kept + [x]", "(kept + [x])[^3..]"),
        text.replace("i32[..3]", "i32[..10]"),
    ] {
        assert_eq!(errors(&compiler, &fixed), vec![], "{fixed}");
    }
}

const BODY_FAULTS: [&str; 7] = [
    "for x in [1, 2] into s from 0: s + true",
    "for x in [1, 2] into s from 0: s + \"a\"",
    "for x in [1, 2] into s from 0: (s + true) * 0",
    "for x in [1, 2] into s from 0: if s then 1 else 2",
    "for x in [1, 2] into s from true: s + 1",
    "for x in [true] into s from 0: s + x",
    "for x in [] into s from 0: s + true",
];

const HOLDERS: [&str; 6] = [
    "let v = V\nClip c:\n    Note/(v)\n",
    "let v: i32 = V\nClip c:\n    Note | velocity=(v * 0.0)\n",
    "pattern P():\n    let v = V\n    Note/(v)\nClip c:\n    P\n",
    "Clip c:\n    Note/([(V)][0])\n",
    "fn r(n): if n < 1 then (V) else r(n - 1)\nClip c:\n    Note/(r(2))\n",
    "fn f():\n    let v = V\n    v\nClip c:\n    Note/(f())\n",
];

#[test]
fn a_for_bodys_type_fault_draws_whatever_holds_the_expression_as_its_derived_cell_twin_draws_c1_1_c2_1()
 {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for value in BODY_FAULTS {
        let twin = format!("state v = {value}\nClip c:\n    Note/(v)\n");
        let expected = support::error_texts(&compiler, &twin);
        if expected.is_empty() {
            failures.push(support::failed_row_with(
                &compiler,
                &twin,
                format!("{twin}\n  the twin draws nothing\n"),
            ));
        }
        for holder in HOLDERS {
            let text = holder.replace('V', value);
            let at = text.find("Clip c:").unwrap_or(0);
            let reversed = format!("{}{}", &text[at..], &text[..at]);
            for program in [text.clone(), reversed] {
                let found = support::error_texts(&compiler, &program);
                if found != expected {
                    failures.push(support::failed_row_with(
                        &compiler,
                        &text,
                        format!("{program}\n  draws {found:?}\n  twin {expected:?}\n"),
                    ));
                }
            }
        }
    }
    for (text, wanted) in [
        (
            "let v = for x in [5, 6] into s from 0: s + x\nClip c:\n    Note/([0][v])\n",
            vec![(Code::IndexOutOfRange, "v".to_string())],
        ),
        (
            "let v = for x in [1] into s from 0.5: s + x\nSynth y:\n    Osc | freq=(v)\n",
            vec![],
        ),
    ] {
        let found = support::error_texts(&compiler, text);
        if found != wanted {
            failures.push(support::failed_row_with(
                &compiler,
                text,
                format!("{text}\n  draws {found:?}\n"),
            ));
        }
    }
    support::assert_no_failure(&failures);
}

#[test]
fn every_range_bound_of_a_for_expression_is_checked_against_i64_before_any_iteration_c1_6_cs_10() {
    let raised = LimitOverrides {
        expansion_steps: Some(1 << 40),
        expansion_steps_warning: Some(1 << 39),
        ..LimitOverrides::default()
    };
    let compilers = [
        Compiler::new(&vocabulary()).expect("the vocabulary compiles"),
        Compiler::with_limits(&vocabulary(), raised).expect("the vocabulary compiles"),
    ];
    let bounds = [
        ("0..2 ** 70", "2 ** 70"),
        ("0..99999999999999999999", "99999999999999999999"),
        ("-(2 ** 70)..0", "-(2 ** 70)"),
        ("0..2 ** 63", "2 ** 63"),
        ("(-(2 ** 63) - 1)..0", "-(2 ** 63) - 1"),
    ];
    let holders = [
        (
            "let v = for i in {b} into s from 0: s\n",
            "Clip c:\n    Note/v\n",
        ),
        (
            "state v = for i in {b} into s from 0: s + n\n",
            "Clip c:\n    Note/v\n",
        ),
        (
            "fn f(k): for i in {b} into s from k: s\n",
            "Clip c:\n    Note/(f(1))\n",
        ),
    ];
    let mut failures = Vec::new();
    for compiler in &compilers {
        for (range, at) in bounds {
            for (holder, clip) in holders {
                for reversed in [false, true] {
                    let mut items = vec!["state n(1)\n".to_string(), holder.replace("{b}", range)];
                    items.push(clip.to_string());
                    if reversed {
                        items.reverse();
                    }
                    let text = items.concat();
                    let found = support::error_texts(compiler, &text);
                    if found != [(Code::RangeBound, at.to_string())] {
                        failures.push(support::failed_row_with(
                            compiler,
                            &text,
                            format!("{text}found {found:?}\n"),
                        ));
                    }
                }
            }
        }
    }
    support::assert_no_failure(&failures);
}
