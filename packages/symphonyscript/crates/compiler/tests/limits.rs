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

fn errors_within(limits: Limits, text: &str) -> Vec<(Code, String)> {
    let compiler = Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
    support::errors(compiler.check(&[support::source(text)]))
        .into_iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.summary))
        .collect()
}

fn follows(code: Code, rows: &[(Limits, String, Option<String>)]) {
    for (limits, text, refusal) in rows {
        let errors = errors_within(*limits, text);
        match refusal {
            None => assert_eq!(errors, vec![], "{limits:?}\n{text}"),
            Some(fragment) => assert!(
                matches!(errors.as_slice(), [(found, message)] if *found == code && message.contains(fragment.as_str())),
                "{errors:?} against {fragment:?} for {limits:?}\n{text}"
            ),
        }
    }
}

fn sizes_totalling(total: u64) -> Vec<u32> {
    let mut remaining = total;
    let mut levels = Vec::new();
    for level in (0..20).rev() {
        let size = (1 << (level + 1)) - 1;
        while remaining >= size {
            remaining -= size;
            levels.push(level);
        }
    }
    levels
}

fn inlining(total: u64) -> String {
    let mut text = String::from("input i = 1\nfn f0(x): x\n");
    for level in 1..20 {
        let below = level - 1;
        text.push_str(&format!("fn f{level}(x): f{below}(x) + f{below}(x)\n"));
    }
    text.push_str("Clip c:\n");
    for level in sizes_totalling(total) {
        text.push_str(&format!("    Note/(f{level}(i))\n"));
    }
    text
}

fn recursing(total: u64) -> String {
    let mut text = String::from(
        "fn g(t, a, b): if a < 1 then 0 else g(t, a - 1, 2 * b) + g(t, a - 1, 2 * b + 1)\nClip c:\n",
    );
    for (tree, level) in sizes_totalling(total).into_iter().enumerate() {
        text.push_str(&format!("    Note/(g({tree}, {level}, 0))\n"));
    }
    text
}

fn nested(calls: u64) -> String {
    format!(
        "fn r(n): if n < 1 then 0 else r(n - 1)\nClip c:\n    Note/(r({}))\n",
        calls - 1
    )
}

fn iterations(count: u64) -> String {
    format!("Clip c:\n    for i in 0..{count}:\n        Note\n")
}

fn empty_iterations(count: u64) -> String {
    format!("Clip c:\n    for i in 0..{count}:\n        for j in 0..0:\n            Note\n")
}

fn uses(count: u64) -> String {
    format!(
        "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n    for i in 0..{count}:\n        P\n"
    )
}

fn wires(count: usize) -> String {
    format!("Clip c:\n{}Clip d\n", "    Plays: d\n".repeat(count))
}

fn refused(fragment: &str) -> Option<String> {
    Some(fragment.to_string())
}

#[test]
fn the_inline_limit_follows_the_compilations_limits() {
    let ten = Limits {
        inlined_calls: 10,
        ..Limits::default()
    };
    let two_million = Limits {
        inlined_calls: 2_000_000,
        ..Limits::default()
    };
    let default = Limits::default();
    let refusal = |bound: &str| refused(&format!("inline more than {bound} calls"));
    follows(
        Code::InlineLimit,
        &[
            (ten, inlining(10), None),
            (ten, inlining(11), refusal("10")),
            (default, inlining(11), None),
            (default, inlining(1_000_000), None),
            (default, inlining(1_000_001), refusal("1,000,000")),
            (default, inlining(1_048_575), refusal("1,000,000")),
            (two_million, inlining(1_048_575), None),
            (two_million, inlining(2_000_000), None),
            (two_million, inlining(2_000_001), refusal("2,000,000")),
        ],
    );
}

#[test]
fn the_nesting_depth_follows_the_compilations_limits() {
    let five = Limits {
        nesting_depth: 5,
        ..Limits::default()
    };
    let fifteen_hundred = Limits {
        nesting_depth: 1_500,
        ..Limits::default()
    };
    let default = Limits::default();
    let refusal = |bound: &str| refused(&format!("deeper than {bound} nested calls"));
    follows(
        Code::RecursionDepth,
        &[
            (five, nested(5), None),
            (five, nested(6), refusal("5")),
            (default, nested(6), None),
            (default, nested(1_024), None),
            (default, nested(1_025), refusal("1,024")),
            (fifteen_hundred, nested(1_500), None),
            (fifteen_hundred, nested(1_501), refusal("1,500")),
        ],
    );
}

#[test]
fn the_recursive_call_limit_follows_the_compilations_limits() {
    let five = Limits {
        recursive_calls: 5,
        ..Limits::default()
    };
    let raised = Limits {
        recursive_calls: 1_048_575,
        ..Limits::default()
    };
    let default = Limits::default();
    let refusal = |bound: &str| refused(&format!("exceeds the {bound} distinct recursive calls"));
    follows(
        Code::RecursionLimit,
        &[
            (five, recursing(5), None),
            (five, recursing(6), refusal("5")),
            (default, recursing(6), None),
            (default, recursing(1_000_000), None),
            (default, recursing(1_000_001), refusal("1,000,000")),
            (raised, recursing(1_048_575), None),
            (raised, recursing(1_048_576), refusal("1,048,575")),
        ],
    );
}

#[test]
fn the_expansion_step_limit_follows_the_compilations_limits() {
    let ten = Limits {
        expansion_steps: 10,
        ..Limits::default()
    };
    let two_million = Limits {
        expansion_steps: 2_000_000,
        ..Limits::default()
    };
    let default = Limits::default();
    let refusal = |bound: &str| {
        refused(&format!(
            "runs more than {bound} `for` iterations and pattern uses"
        ))
    };
    follows(
        Code::ExpansionLimit,
        &[
            (ten, iterations(10), None),
            (ten, iterations(11), refusal("10")),
            (ten, uses(5), None),
            (ten, uses(6), refusal("10")),
            (default, iterations(11), None),
            (default, empty_iterations(1_000_000), None),
            (default, empty_iterations(1_000_001), refusal("1,000,000")),
            (two_million, empty_iterations(2_000_000), None),
            (
                two_million,
                empty_iterations(2_000_001),
                refusal("2,000,000"),
            ),
        ],
    );
}

#[test]
fn the_expanded_item_limit_follows_the_compilations_limits() {
    let ten = Limits {
        expanded_items: 10,
        ..Limits::default()
    };
    let raised = Limits {
        expanded_items: 1_000_001,
        expansion_steps: 2_000_000,
        ..Limits::default()
    };
    let default = Limits::default();
    let refusal = |bound: &str| refused(&format!("yields more than {bound} instances and wires"));
    follows(
        Code::ExpansionLimit,
        &[
            (ten, iterations(9), None),
            (ten, iterations(10), refusal("10")),
            (ten, wires(8), None),
            (ten, wires(9), refusal("10")),
            (default, iterations(10), None),
            (default, iterations(999_999), None),
            (default, iterations(1_000_000), refusal("1,000,000")),
            (raised, iterations(1_000_000), None),
            (raised, iterations(1_000_001), refusal("1,000,001")),
        ],
    );
}

#[test]
fn a_stopped_compilation_reports_its_one_limit_and_the_work_it_finished() {
    let flow = "Session s\nlet a = Sine\nKit k:\n    Sampler | file=\"{a}\"\n";
    let typed_past_items = format!("{flow}Synth y:\n    Osc | wave=a\n");
    let typed_past_steps = format!("{flow}Synth y:\n    for j in 0..1:\n        Osc | wave=a\n");
    let typed_past_inlining = format!("{flow}fn g(x): x\nSynth y:\n    Osc | wave=g(a)\n");
    let recursion = format!(
        "{flow}fn r(n): if n < 1 then 0 else r(n - 1)\nClip c:\n    Note/(r(4))\nSynth y:\n    Osc | wave=a\n"
    );
    let unsupplied = "pattern P(n):\n    Note\nClip d:\n    P\n";
    let unfitting = "Clip e:\n    Note/300000000000\n";
    let items = |value| Limits {
        expanded_items: value,
        ..Limits::default()
    };
    let steps = |value| Limits {
        expansion_steps: value,
        ..Limits::default()
    };
    let inlined = |value| Limits {
        inlined_calls: value,
        ..Limits::default()
    };
    let recursive = |value| Limits {
        recursive_calls: value,
        ..Limits::default()
    };
    let rows = [
        (
            items(4),
            typed_past_items.clone(),
            vec![Code::ExpansionLimit],
        ),
        (items(5), typed_past_items.clone(), vec![]),
        (items(6), typed_past_items, vec![]),
        (
            steps(0),
            typed_past_steps.clone(),
            vec![Code::ExpansionLimit],
        ),
        (steps(1), typed_past_steps, vec![]),
        (
            inlined(0),
            typed_past_inlining.clone(),
            vec![Code::InlineLimit],
        ),
        (inlined(1), typed_past_inlining, vec![]),
        (recursive(4), recursion.clone(), vec![Code::RecursionLimit]),
        (recursive(5), recursion.clone(), vec![]),
        (
            recursive(4),
            format!("{recursion}{unsupplied}"),
            vec![Code::RecursionLimit, Code::MissingArgument],
        ),
        (
            recursive(4),
            format!("{recursion}{unfitting}"),
            vec![Code::RecursionLimit],
        ),
        (
            recursive(4),
            format!("{unfitting}{recursion}"),
            vec![Code::DoesNotFit, Code::RecursionLimit],
        ),
        (
            items(4),
            format!("{unsupplied}Session s\nKit k:\n    Sampler | file=\"x\"\nSynth y\n"),
            vec![Code::ExpansionLimit, Code::MissingArgument],
        ),
    ];
    for (limits, text, expected) in rows {
        let codes: Vec<Code> = errors_within(limits, &text)
            .into_iter()
            .map(|(code, _)| code)
            .collect();
        assert_eq!(codes, expected, "{limits:?}\n{text}");
    }
}
