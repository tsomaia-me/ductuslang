mod support;

use std::time::{Duration, Instant};

use symphonyscript::{Code, Compiler, Severity, Source};
use symphonyscript_core::{Count, Limits, Vocabulary};
use symphonyscript_fixtures::vocabulary;

fn codes(text: &str) -> Vec<Code> {
    support::checked(text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn a_million_iterations_and_uses_in_all_are_accepted_and_one_more_is_not() {
    let million = "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n    for i in 0..500000:\n        P\n";
    assert_eq!(codes(million), vec![]);
    let one_more =
        format!("{million}    for k in 0..1:\n        for j in 0..0:\n            Note\n");
    assert_eq!(codes(&one_more), vec![Code::ExpansionLimit]);
}

#[test]
fn an_integer_that_would_reach_two_to_the_127_is_reported_before_it_is_computed() {
    let just_under = "170141183460469231731687303715884105727";
    let reference = "Clip c:\n    Note/(2 ** 127)\n";
    assert_eq!(codes(reference), vec![Code::IntegerOverflow]);
    let programs = [
        "Clip c:\n    Note/(2 ** 1000000000)\n".to_string(),
        "let e = 2 ** 2 ** 2 ** 2 ** 2\nClip c:\n    Note/e\n".to_string(),
        format!("Clip c:\n    Note/({just_under} * {just_under})\n"),
    ];
    for text in &programs {
        assert_eq!(codes(text), vec![Code::IntegerOverflow], "{text}");
        let (time, reference_time) = best_times(text, reference);
        assert!(
            time <= reference_time * 10,
            "{time:?} against {reference_time:?}: {text}"
        );
    }
}

fn best_times(text: &str, reference: &str) -> (Duration, Duration) {
    let timed = |text: &str| {
        let start = Instant::now();
        for _ in 0..10 {
            support::checked(text);
        }
        start.elapsed()
    };
    let (mut best, mut best_reference) = (Duration::MAX, Duration::MAX);
    for _ in 0..5 {
        best = best.min(timed(text));
        best_reference = best_reference.min(timed(reference));
    }
    (best, best_reference)
}

#[test]
fn variant_lookups_cost_constant_time_per_name() {
    let compiler_over = |names: Vec<String>| {
        let mut v = Vocabulary::builder("wide");
        let variants = v.enumeration("E", &names);
        let other = v.enumeration("F", ["Other"]);
        let kind = v.node("K");
        v.kind(kind)
            .top_level(Count::Any)
            .attr_enum("e", variants, "V65535")
            .attr_enum("f", other, "Other");
        let vocabulary = v.build().expect("the vocabulary builds");
        Compiler::new(&vocabulary).expect("the vocabulary compiles")
    };
    let named = |range: std::ops::Range<usize>| range.map(|index| format!("V{index}")).collect();
    let wide = compiler_over(named(0..65_536));
    let narrow = compiler_over(named(65_472..65_536));
    let mut text = String::new();
    for index in 0..20_000 {
        text.push_str(&format!("K k{index} | e=V{}\n", 65_472 + index % 64));
    }
    text.push_str("K x | e=Other\n");
    let source = [Source {
        path: "wide.sym".into(),
        text,
    }];
    let errors = |compiler: &Compiler| -> Vec<Code> {
        compiler
            .check(&source)
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
            .map(|diagnostic| diagnostic.code)
            .collect()
    };
    assert_eq!(errors(&wide), vec![Code::UnknownName]);
    assert_eq!(errors(&narrow), vec![Code::UnknownName]);
    let timed = |compiler: &Compiler| {
        let start = Instant::now();
        compiler.check(&source);
        start.elapsed()
    };
    let (mut wide_time, mut narrow_time) = (Duration::MAX, Duration::MAX);
    for _ in 0..3 {
        wide_time = wide_time.min(timed(&wide));
        narrow_time = narrow_time.min(timed(&narrow));
    }
    assert!(
        wide_time <= narrow_time * 3,
        "{wide_time:?} over 65,536 variants against {narrow_time:?} over 64"
    );
}

fn errors_within(limits: Limits, text: &str) -> Vec<Code> {
    let compiler = Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
    support::errors(compiler.check(&[support::source(text)]))
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn repeated(line: &str, times: usize) -> String {
    line.repeat(times)
}

fn doubling_chain(levels: usize) -> String {
    let mut text = String::from("input i = 1\nfn f0(x): x\n");
    for level in 1..=levels {
        let below = level - 1;
        text.push_str(&format!("fn f{level}(x): f{below}(x) + f{below}(x)\n"));
    }
    text.push_str(&format!("Clip c:\n    Note/(f{levels}(i))\n"));
    text
}

#[test]
fn the_inline_limit_follows_the_compilations_limits() {
    let calls = |count: usize| {
        format!(
            "input i = 1\nfn f(x): x\nClip c:\n{}",
            repeated("    Note/(f(i))\n", count)
        )
    };
    let ten = Limits {
        inlined_calls: 10,
        ..Limits::default()
    };
    let two_million = Limits {
        inlined_calls: 2_000_000,
        ..Limits::default()
    };
    let rows = [
        (ten, calls(10), vec![]),
        (ten, calls(11), vec![Code::InlineLimit]),
        (Limits::default(), calls(11), vec![]),
        (
            Limits::default(),
            doubling_chain(19),
            vec![Code::InlineLimit],
        ),
        (two_million, doubling_chain(19), vec![]),
    ];
    for (limits, text, expected) in rows {
        assert_eq!(errors_within(limits, &text), expected, "{limits:?}\n{text}");
    }
}

#[test]
fn the_nesting_depth_follows_the_compilations_limits() {
    let deep = |calls: usize| {
        format!(
            "fn r(n): if n < 1 then 0 else r(n - 1)\nClip c:\n    Note/(r({}))\n",
            calls - 1
        )
    };
    let five = Limits {
        nesting_depth: 5,
        ..Limits::default()
    };
    let fifteen_hundred = Limits {
        nesting_depth: 1_500,
        ..Limits::default()
    };
    let rows = [
        (five, deep(5), vec![]),
        (five, deep(6), vec![Code::RecursionDepth]),
        (Limits::default(), deep(6), vec![]),
        (Limits::default(), deep(1_024), vec![]),
        (Limits::default(), deep(1_025), vec![Code::RecursionDepth]),
        (fifteen_hundred, deep(1_500), vec![]),
        (fifteen_hundred, deep(1_501), vec![Code::RecursionDepth]),
    ];
    for (limits, text, expected) in rows {
        assert_eq!(errors_within(limits, &text), expected, "{limits:?}\n{text}");
    }
}

#[test]
fn the_recursive_call_limit_follows_the_compilations_limits() {
    let chain = |calls: usize| {
        format!(
            "fn r(n): if n < 1 then 0 else r(n - 1)\nClip c:\n    Note/(r({}))\n",
            calls - 1
        )
    };
    let tree = "fn g(a, b): if a < 1 then 0 else g(a - 1, 2 * b) + g(a - 1, 2 * b + 1)\nClip c:\n    Note/(g(19, 0))\n".to_string();
    let five = Limits {
        recursive_calls: 5,
        ..Limits::default()
    };
    let whole_tree = Limits {
        recursive_calls: 1_048_575,
        ..Limits::default()
    };
    let rows = [
        (five, chain(5), vec![]),
        (five, chain(6), vec![Code::RecursionLimit]),
        (Limits::default(), chain(6), vec![]),
        (Limits::default(), tree.clone(), vec![Code::RecursionLimit]),
        (whole_tree, tree, vec![]),
    ];
    for (limits, text, expected) in rows {
        assert_eq!(errors_within(limits, &text), expected, "{limits:?}\n{text}");
    }
}

#[test]
fn the_expansion_step_limit_follows_the_compilations_limits() {
    let iterations = |count: usize| format!("Clip c:\n    for i in 0..{count}:\n        Note\n");
    let uses = |count: usize| {
        format!(
            "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n    for i in 0..{count}:\n        P\n"
        )
    };
    let ten = Limits {
        expansion_steps: 10,
        ..Limits::default()
    };
    let two_million = Limits {
        expansion_steps: 2_000_000,
        ..Limits::default()
    };
    let rows = [
        (ten, iterations(10), vec![]),
        (ten, iterations(11), vec![Code::ExpansionLimit]),
        (ten, uses(5), vec![]),
        (ten, uses(6), vec![Code::ExpansionLimit]),
        (Limits::default(), iterations(11), vec![]),
        (
            Limits::default(),
            uses(1_000_000),
            vec![Code::ExpansionLimit],
        ),
        (two_million, uses(1_000_000), vec![]),
        (two_million, uses(1_000_001), vec![Code::ExpansionLimit]),
    ];
    for (limits, text, expected) in rows {
        assert_eq!(errors_within(limits, &text), expected, "{limits:?}\n{text}");
    }
}

#[test]
fn the_expanded_item_limit_follows_the_compilations_limits() {
    let notes = |count: usize| format!("Clip c:\n    for i in 0..{count}:\n        Note\n");
    let wires = |count: usize| format!("Clip c:\n{}Clip d\n", repeated("    Plays: d\n", count));
    let ten = Limits {
        expanded_items: 10,
        ..Limits::default()
    };
    let million_and_one = Limits {
        expanded_items: 1_000_001,
        ..Limits::default()
    };
    let rows = [
        (ten, notes(9), vec![]),
        (ten, notes(10), vec![Code::ExpansionLimit]),
        (ten, wires(8), vec![]),
        (ten, wires(9), vec![Code::ExpansionLimit]),
        (Limits::default(), notes(10), vec![]),
        (
            Limits::default(),
            notes(1_000_000),
            vec![Code::ExpansionLimit],
        ),
        (million_and_one, notes(1_000_000), vec![]),
        (
            million_and_one,
            notes(1_000_001),
            vec![Code::ExpansionLimit],
        ),
    ];
    for (limits, text, expected) in rows {
        assert_eq!(errors_within(limits, &text), expected, "{limits:?}\n{text}");
    }
}
