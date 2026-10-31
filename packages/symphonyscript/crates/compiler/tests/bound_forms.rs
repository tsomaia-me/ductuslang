mod support;

use symphonyscript::{Code, Compiler, Diagnostic};
use symphonyscript_fixtures::{Rows, covered, vocabulary};

const CELLS: &str = "state a([]): i32[..5]\nstate i(1)\nlet k = 4\n";

const FORMS: [&str; 16] = [
    "4",
    "k",
    "len(a)",
    "min(i, 4)",
    "max(i, 2)",
    "len(a) + 1",
    "len(a) - 2",
    "min(len(a), 4) - max(i, 1)",
    "max(min(i, 3), 1)",
    "i + 2",
    "i",
    "max(i, 2) + 1",
    "min(i, 4) + 1",
    "4 - min(i, 1)",
    "4 - max(i, 2)",
    "16 - i",
];

const RANGES: [(Option<i128>, Option<i128>); 16] = [
    (Some(4), Some(4)),
    (Some(4), Some(4)),
    (Some(0), Some(5)),
    (None, Some(4)),
    (Some(2), None),
    (Some(1), Some(6)),
    (Some(-2), Some(3)),
    (None, Some(3)),
    (Some(1), Some(3)),
    (None, None),
    (None, None),
    (Some(3), None),
    (None, Some(5)),
    (Some(3), None),
    (None, Some(2)),
    (None, None),
];

const CONTEXTS: [&str; 5] = [
    "a[..F]",
    "a[F..]",
    "for x in 0..F into s from []: s + [x + i]",
    "for x in F..6 into s from []: s + [x + i]",
    "placement",
];

const POSITIONS: [&str; 2] = ["state v = E\n", "fn g(p): E\nstate v = g(0)\n"];

enum Expected {
    Capacity(i128),
    BoundError,
    Codes(Vec<Code>),
}

fn expected(form: usize, context: usize) -> Expected {
    let (min, max) = RANGES[form];
    let compile_time = form < 2;
    match context {
        0 => Expected::Capacity(max.map_or(5, |max| max.clamp(0, 5))),
        1 => Expected::Capacity(5 - min.map_or(0, |min| min.clamp(0, 5))),
        2 => match max {
            Some(max) => Expected::Capacity(max.max(0)),
            None => Expected::BoundError,
        },
        3 => match min {
            Some(min) => Expected::Capacity((6 - min).max(0)),
            None => Expected::BoundError,
        },
        _ if compile_time => Expected::Codes(Vec::new()),
        _ => Expected::Codes(vec![Code::ReactiveIterable]),
    }
}

fn errors(compiler: &Compiler, text: &str) -> Vec<Diagnostic> {
    support::errors(compiler.check(&[support::source(text)]))
}

fn codes(compiler: &Compiler, text: &str) -> Vec<Code> {
    errors(compiler, text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn held_within(compiler: &Compiler, declared: &str, capacity: i128) -> Result<(), String> {
    let probe = |kept: i128| {
        format!("{declared}state probe: i32[..{kept}] = v\nClip c:\n    Note/(probe[0])\n")
    };
    let kept = probe(capacity.max(1));
    let found = codes(compiler, &kept);
    if !found.is_empty() {
        return Err(format!("{kept}\n  draws {found:?}\n"));
    }
    if capacity < 2 {
        return Ok(());
    }
    let below = probe(capacity - 1);
    let found = codes(compiler, &below);
    if found == [Code::CapacityExceeded] {
        Ok(())
    } else {
        Err(format!(
            "{below}\n  draws {found:?}, not capacity-exceeded\n"
        ))
    }
}

fn bound_rows(rows: &Rows) -> (Vec<String>, usize) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    let mut programs = 0;
    for (form, written) in FORMS.iter().enumerate() {
        for (context, shape) in CONTEXTS.iter().enumerate() {
            for (position, template) in POSITIONS.iter().enumerate() {
                if !rows.take(&[form, context, position]) {
                    continue;
                }
                programs += 1;
                let outcome = if context == 4 {
                    let text = format!(
                        "{CELLS}{}Clip c:\n    for x in 0..{written}:\n        Note\n",
                        template.replace('E', "0")
                    );
                    match expected(form, context) {
                        Expected::Codes(expected) if codes(&compiler, &text) == expected => Ok(()),
                        Expected::Codes(expected) => Err(format!(
                            "{text}\n  draws {:?}, not {expected:?}\n",
                            codes(&compiler, &text)
                        )),
                        Expected::Capacity(_) | Expected::BoundError => {
                            Err("a placement row expects codes".to_string())
                        }
                    }
                } else {
                    let value = shape.replace('F', written);
                    let declared = format!("{CELLS}{}", template.replace('E', &value));
                    match expected(form, context) {
                        Expected::Capacity(capacity) => held_within(&compiler, &declared, capacity),
                        Expected::BoundError => {
                            let text = format!("{declared}Clip c:\n    Note/(v[0])\n");
                            let found = errors(&compiler, &text);
                            let at = |diagnostic: &Diagnostic| {
                                let range = &diagnostic.span.range;
                                text.get(range.start as usize..range.end as usize)
                                    .unwrap_or_default()
                                    .to_string()
                            };
                            match found.as_slice() {
                                [diagnostic]
                                    if diagnostic.code == Code::RangeBound
                                        && at(diagnostic) == *written =>
                                {
                                    Ok(())
                                }
                                _ => Err(format!("{text}\n  draws {found:?}, not range-bound\n")),
                            }
                        }
                        Expected::Codes(_) => Err("a value row expects a capacity".to_string()),
                    }
                };
                failures.extend(outcome.err());
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
fn every_bound_form_gives_a_slice_and_a_range_the_minimum_and_maximum_its_form_reads() {
    assert_none_fail(covered(bound_rows), 80);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_every_bound_form_gives_a_slice_and_a_range_the_minimum_and_maximum_its_form_reads() {
    assert_none_fail(bound_rows(&Rows::Every), 160);
}

#[test]
fn the_bounds_of_the_spec_give_their_capacities_and_a_range_bound_without_a_maximum_is_rejected() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let held = "state xs([]): i32[..10]\nstate n(3)\n";
    for (value, capacity) in [("xs[^8..]", 8), ("xs[1..]", 9), ("xs[..min(n, 4)]", 4)] {
        let declared = format!("{held}state v = {value}\n");
        assert_eq!(
            held_within(&compiler, &declared, capacity),
            Ok(()),
            "{value}"
        );
    }
    let text = "state n(4)\nstate total = for i in 0..n into sum from 0: sum + i\nClip c:\n    Note/total\n";
    let found = errors(&compiler, text);
    assert!(
        matches!(found.as_slice(), [diagnostic]
            if diagnostic.code == Code::RangeBound
                && diagnostic.help.as_deref() == Some("bound it by its form, as in `0..min(n, 16)`")),
        "{found:?}"
    );
    let fixed = text.replace("0..n", "0..min(n, 16)");
    assert_eq!(errors(&compiler, &fixed), vec![], "{fixed}");
    for (range, bound) in [
        ("0..16 - n", "16 - n"),
        ("16 - n..20", "16 - n"),
        ("min(n, 16)..20", "min(n, 16)"),
    ] {
        let unbounded = text.replace("0..n", range);
        let found = errors(&compiler, &unbounded);
        assert!(
            matches!(found.as_slice(), [diagnostic]
                if diagnostic.code == Code::RangeBound
                    && unbounded.get(diagnostic.span.range.start as usize..diagnostic.span.range.end as usize) == Some(bound)),
            "{unbounded}\n  draws {found:?}"
        );
    }
}
