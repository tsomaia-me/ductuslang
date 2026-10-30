mod support;

use symphonyscript::{Code, Compiler, Diagnostic, Severity, Source};
use symphonyscript_core::{Count, LimitOverrides, Vocabulary};
use symphonyscript_fixtures::{Rows, covered, vocabulary};

fn codes(text: &str) -> Vec<Code> {
    support::checked(text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn a_limits_worth_of_iterations_and_uses_in_all_is_accepted_and_one_more_is_not() {
    let thousand = LimitOverrides {
        expansion_steps: Some(1_000),
        ..LimitOverrides::default()
    };
    let codes_within = |text: &str| -> Vec<Code> {
        errors_within(thousand, text)
            .into_iter()
            .map(|(code, _)| code)
            .collect()
    };
    let limit = "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n    for i in 0..500:\n        P\n";
    assert_eq!(codes_within(limit), vec![]);
    let one_more = format!("{limit}    for k in 0..1:\n        for j in 0..0:\n            Note\n");
    assert_eq!(codes_within(&one_more), vec![Code::ExpansionLimit]);
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
    }
}

#[test]
#[ignore = "stress"]
fn stress_a_variant_is_found_among_any_number_of_variants() {
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
}

fn errors_within(limits: LimitOverrides, text: &str) -> Vec<(Code, String)> {
    let compiler = Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
    support::errors(compiler.check(&[support::source(text)]))
        .into_iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.summary))
        .collect()
}

fn follows(code: Code, rows: &[(LimitOverrides, String, Option<String>)]) {
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

type Helped = (Code, String, Option<String>);

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
    let ten = LimitOverrides {
        inlined_calls: Some(10),
        ..LimitOverrides::default()
    };
    let thousand = LimitOverrides {
        inlined_calls: Some(1_000),
        ..LimitOverrides::default()
    };
    let two_million = LimitOverrides {
        inlined_calls: Some(2_000_000),
        ..LimitOverrides::default()
    };
    let default = LimitOverrides::default();
    let refusal = |bound: &str| refused(&format!("inline more than {bound} calls"));
    follows(
        Code::InlineLimit,
        &[
            (ten, inlining(10), None),
            (ten, inlining(11), refusal("10")),
            (default, inlining(11), None),
            (thousand, inlining(1_000), None),
            (thousand, inlining(1_001), refusal("1,000")),
            (thousand, inlining(2_047), refusal("1,000")),
            (default, inlining(1_000_001), refusal("1,000,000")),
            (two_million, inlining(2_000_001), refusal("2,000,000")),
        ],
    );
}

#[test]
fn the_nesting_depth_follows_the_compilations_limits() {
    let five = LimitOverrides {
        nesting_depth: Some(5),
        ..LimitOverrides::default()
    };
    let fifteen_hundred = LimitOverrides {
        nesting_depth: Some(1_500),
        ..LimitOverrides::default()
    };
    let default = LimitOverrides::default();
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
    let five = LimitOverrides {
        recursive_calls: Some(5),
        ..LimitOverrides::default()
    };
    let thousand = LimitOverrides {
        recursive_calls: Some(1_000),
        ..LimitOverrides::default()
    };
    let default = LimitOverrides::default();
    let refusal = |bound: &str| refused(&format!("exceeds the {bound} distinct recursive calls"));
    follows(
        Code::RecursionLimit,
        &[
            (five, recursing(5), None),
            (five, recursing(6), refusal("5")),
            (default, recursing(6), None),
            (thousand, recursing(1_000), None),
            (thousand, recursing(1_001), refusal("1,000")),
        ],
    );
}

#[test]
#[ignore = "stress"]
fn stress_the_recursive_call_limit_refuses_the_call_past_its_default() {
    follows(
        Code::RecursionLimit,
        &[(
            LimitOverrides::default(),
            recursing(1_000_001),
            refused("exceeds the 1,000,000 distinct recursive calls"),
        )],
    );
}

#[test]
#[ignore = "stress"]
fn stress_the_recursive_call_limit_refuses_the_call_past_a_raised_limit() {
    let raised = LimitOverrides {
        recursive_calls: Some(1_048_575),
        ..LimitOverrides::default()
    };
    follows(
        Code::RecursionLimit,
        &[(
            raised,
            recursing(1_048_576),
            refused("exceeds the 1,048,575 distinct recursive calls"),
        )],
    );
}

#[test]
fn the_expansion_step_limit_follows_the_compilations_limits() {
    let ten = LimitOverrides {
        expansion_steps: Some(10),
        ..LimitOverrides::default()
    };
    let thousand = LimitOverrides {
        expansion_steps: Some(1_000),
        ..LimitOverrides::default()
    };
    let default = LimitOverrides::default();
    follows(
        Code::ExpansionLimit,
        &[
            (ten, iterations(10), None),
            (ten, iterations(11), expansion_refusal("10")),
            (ten, uses(5), None),
            (ten, uses(6), expansion_refusal("10")),
            (default, iterations(11), None),
            (thousand, empty_iterations(1_000), None),
            (
                thousand,
                empty_iterations(1_001),
                expansion_refusal("1,000"),
            ),
        ],
    );
}

fn expansion_refusal(bound: &str) -> Option<String> {
    refused(&format!(
        "runs more than {bound} `for` iterations and pattern uses"
    ))
}

#[test]
#[ignore = "stress"]
fn stress_the_expansion_step_limit_refuses_the_step_past_its_default_and_a_raised_limit() {
    let two_million = LimitOverrides {
        expansion_steps: Some(2_000_000),
        ..LimitOverrides::default()
    };
    follows(
        Code::ExpansionLimit,
        &[
            (
                LimitOverrides::default(),
                empty_iterations(1_000_001),
                expansion_refusal("1,000,000"),
            ),
            (
                two_million,
                empty_iterations(2_000_001),
                expansion_refusal("2,000,000"),
            ),
        ],
    );
}

#[test]
fn the_expanded_item_limit_follows_the_compilations_limits() {
    let ten = LimitOverrides {
        expanded_items: Some(10),
        ..LimitOverrides::default()
    };
    let thousand = LimitOverrides {
        expanded_items: Some(1_000),
        ..LimitOverrides::default()
    };
    let default = LimitOverrides::default();
    follows(
        Code::ExpansionLimit,
        &[
            (ten, iterations(9), None),
            (ten, iterations(10), item_refusal("10")),
            (ten, wires(8), None),
            (ten, wires(9), item_refusal("10")),
            (default, iterations(10), None),
            (thousand, iterations(999), None),
            (thousand, iterations(1_000), item_refusal("1,000")),
        ],
    );
}

fn item_refusal(bound: &str) -> Option<String> {
    refused(&format!("yields more than {bound} instances and wires"))
}

#[test]
#[ignore = "stress"]
fn stress_the_expanded_item_limit_refuses_the_item_past_its_default_and_a_raised_limit() {
    let raised = LimitOverrides {
        expanded_items: Some(1_000_001),
        expansion_steps: Some(2_000_000),
        ..LimitOverrides::default()
    };
    follows(
        Code::ExpansionLimit,
        &[
            (
                LimitOverrides::default(),
                iterations(1_000_000),
                item_refusal("1,000,000"),
            ),
            (raised, iterations(1_000_001), item_refusal("1,000,001")),
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
    let items = |value| LimitOverrides {
        expanded_items: Some(value),
        ..LimitOverrides::default()
    };
    let steps = |value| LimitOverrides {
        expansion_steps: Some(value),
        ..LimitOverrides::default()
    };
    let inlined = |value| LimitOverrides {
        inlined_calls: Some(value),
        ..LimitOverrides::default()
    };
    let recursive = |value| LimitOverrides {
        recursive_calls: Some(value),
        ..LimitOverrides::default()
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

fn a_limit_reached_through_a_default_that_calls_its_function_names_the_default_in_its_help_in(
    orders: support::Orders,
) {
    let thousand = LimitOverrides {
        recursive_calls: Some(1_000),
        inlined_calls: Some(1_000),
        ..LimitOverrides::default()
    };
    let compiler = Compiler::with_limits(&vocabulary(), thousand).expect("the vocabulary compiles");
    let helps = |items: &[&str]| -> Vec<Vec<Helped>> {
        orders
            .of(items)
            .into_iter()
            .map(|order| {
                let text = order.concat();
                support::errors(compiler.check(&[support::source(&text)]))
                    .into_iter()
                    .map(|diagnostic| {
                        let range = diagnostic.span.range.start as usize
                            ..diagnostic.span.range.end as usize;
                        let spanned = text.get(range).unwrap_or_default().to_string();
                        (diagnostic.code, spanned, diagnostic.help)
                    })
                    .collect()
            })
            .collect()
    };
    let named = |code: Code, call: &str| {
        vec![(
            code,
            call.to_string(),
            Some(String::from(
                "pass `y` in the call, or give it a default that does not call `f`, since `y`'s default makes every call of `f` another",
            )),
        )]
    };
    let equal = "fn f(x, y = f(x)): x\n";
    let rising = "fn f(x, y = f(x + 1)): x + y\n";
    let compile_time = "Clip c:\n    Note/(f(1) + 60)\n";
    let rows: [(&[&str], Vec<Helped>); 3] = [
        (
            &[rising, compile_time],
            named(Code::RecursionLimit, "f(x + 1)"),
        ),
        (&[equal, "Clip c:\n    Note/(f(1, 0) + 60)\n"], vec![]),
        (
            &[
                "fn f(x, y = x + 1): if x > 0 then f(x - 1) + y else y\n",
                "Clip c:\n    Note/(f(2) + 60)\n",
            ],
            vec![],
        ),
    ];
    for (items, expected) in rows {
        for found in helps(items) {
            assert_eq!(found, expected, "{items:?}");
        }
    }
    let varying = |limits: LimitOverrides, text: &str, code: Code, help: &str| {
        let compiler =
            Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
        let found: Vec<(Code, Option<String>)> =
            support::errors(compiler.check(&[support::source(text)]))
                .into_iter()
                .map(|diagnostic| (diagnostic.code, diagnostic.help))
                .collect();
        assert_eq!(found, vec![(code, Some(help.to_string()))], "{text}");
    };
    varying(
        LimitOverrides {
            recursive_calls: Some(5),
            ..LimitOverrides::default()
        },
        &recursing(6),
        Code::RecursionLimit,
        "reduce the distinct arguments the recursion computes",
    );
    varying(
        LimitOverrides {
            inlined_calls: Some(10),
            ..LimitOverrides::default()
        },
        &inlining(11),
        Code::InlineLimit,
        "compute a repeated value once, as in `h0(x) * 10`, or lower the bound",
    );
}

#[test]
fn a_limit_reached_through_a_default_that_calls_its_function_names_the_default_in_its_help() {
    a_limit_reached_through_a_default_that_calls_its_function_names_the_default_in_its_help_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_limit_reached_through_a_default_that_calls_its_function_names_the_default_in_its_help()
 {
    a_limit_reached_through_a_default_that_calls_its_function_names_the_default_in_its_help_in(
        support::Orders::Every,
    );
}

struct Counted {
    key: &'static str,
    program: fn(u64) -> String,
    error: Code,
    counted: fn(u64) -> String,
}

fn counted_sites() -> [Counted; 9] {
    [
        Counted {
            key: "recursive_calls",
            program: recursing,
            error: Code::RecursionLimit,
            counted: |warning| {
                format!("this call of `g` exceeds {warning} distinct recursive calls")
            },
        },
        Counted {
            key: "nesting_depth",
            program: nested,
            error: Code::RecursionDepth,
            counted: |warning| format!("the evaluation goes deeper than {warning} nested calls"),
        },
        Counted {
            key: "inlined_calls",
            program: inlining,
            error: Code::InlineLimit,
            counted: |warning| format!("makes the compilation inline more than {warning} calls"),
        },
        Counted {
            key: "inlined_calls",
            program: inlining_into_a_let,
            error: Code::InlineLimit,
            counted: |warning| format!("makes the compilation inline more than {warning} calls"),
        },
        Counted {
            key: "inlined_calls",
            program: inlining_into_a_bound,
            error: Code::InlineLimit,
            counted: |warning| format!("makes the compilation inline more than {warning} calls"),
        },
        Counted {
            key: "expansion_steps",
            program: iterations,
            error: Code::ExpansionLimit,
            counted: |warning| {
                format!("the expansion runs more than {warning} `for` iterations and pattern uses")
            },
        },
        Counted {
            key: "expansion_steps",
            program: uses_in_a_body,
            error: Code::ExpansionLimit,
            counted: |warning| {
                format!("the expansion runs more than {warning} `for` iterations and pattern uses")
            },
        },
        Counted {
            key: "expanded_items",
            program: |count| iterations(count - 1),
            error: Code::ExpansionLimit,
            counted: |warning| {
                format!("the expansion yields more than {warning} instances and wires")
            },
        },
        Counted {
            key: "expanded_items",
            program: |count| wires(count as usize - 2),
            error: Code::ExpansionLimit,
            counted: |warning| {
                format!("the expansion yields more than {warning} instances and wires")
            },
        },
    ]
}

fn inlining_into_a_let(total: u64) -> String {
    let mut text = String::from("input i = 1\nfn f0(x): x\n");
    for level in 1..20 {
        let below = level - 1;
        text.push_str(&format!("fn f{level}(x): f{below}(x) + f{below}(x)\n"));
    }
    let calls: Vec<String> = sizes_totalling(total)
        .into_iter()
        .map(|level| format!("f{level}(i)"))
        .collect();
    text.push_str(&format!(
        "let v = {}\nClip c:\n    Note/(v)\n",
        calls.join(" + ")
    ));
    text
}

fn inlining_into_a_bound(total: u64) -> String {
    let calls = vec!["f(0)"; total as usize].join(" + ");
    format!("fn f(x): x\nClip c:\n    for k in 0..({calls}):\n        Note\n")
}

fn uses_in_a_body(count: u64) -> String {
    format!(
        "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n{}",
        "    P\n".repeat(count as usize)
    )
}

fn limit_and_warning(key: &str, limit: Option<u64>, warning: Option<u64>) -> LimitOverrides {
    let mut overrides = LimitOverrides::default();
    let (limit_field, warning_field) = match key {
        "recursive_calls" => (
            &mut overrides.recursive_calls,
            &mut overrides.recursive_calls_warning,
        ),
        "nesting_depth" => (
            &mut overrides.nesting_depth,
            &mut overrides.nesting_depth_warning,
        ),
        "inlined_calls" => (
            &mut overrides.inlined_calls,
            &mut overrides.inlined_calls_warning,
        ),
        "expansion_steps" => (
            &mut overrides.expansion_steps,
            &mut overrides.expansion_steps_warning,
        ),
        "expanded_items" => (
            &mut overrides.expanded_items,
            &mut overrides.expanded_items_warning,
        ),
        other => panic!("no limit {other}"),
    };
    *limit_field = limit;
    *warning_field = warning;
    overrides
}

fn checked_within(overrides: LimitOverrides, text: &str) -> Vec<Diagnostic> {
    Compiler::with_limits(&vocabulary(), overrides)
        .expect("the vocabulary compiles")
        .check(&[support::source(text)])
}

const UNFITTING_INPUT: &str = "input wide: u8 = 300\n";

fn a_count_past_its_warning_value_draws_the_warning_where_the_limit_would_draw_its_error_in(
    rows: &Rows,
) -> (Vec<String>, usize) {
    let limit = 10;
    let mut failures = Vec::new();
    let mut taken = 0;
    for (site_tag, site) in counted_sites().iter().enumerate() {
        for (warning_tag, warning) in [5, 10, 15].into_iter().enumerate() {
            for (count_tag, count) in [4, 7, 12].into_iter().enumerate() {
                for (beside_tag, beside) in ["", UNFITTING_INPUT].into_iter().enumerate() {
                    if (count > limit && !beside.is_empty())
                        || !rows.take(&[site_tag, warning_tag, count_tag, beside_tag])
                    {
                        continue;
                    }
                    taken += 1;
                    let text = format!("{beside}{}", (site.program)(count));
                    let found = checked_within(
                        limit_and_warning(site.key, Some(limit), Some(warning)),
                        &text,
                    );
                    let warns = warning < limit && count > warning;
                    let mut expected: Vec<Code> = Vec::new();
                    if !beside.is_empty() {
                        expected.push(Code::DoesNotFit);
                    }
                    if warns {
                        expected.push(Code::LimitWarning);
                    }
                    if count > limit {
                        expected.push(site.error);
                    }
                    expected.sort();
                    let mut codes: Vec<Code> = found
                        .iter()
                        .filter(|diagnostic| {
                            diagnostic.severity() == Severity::Error
                                || diagnostic.code == Code::LimitWarning
                        })
                        .map(|diagnostic| diagnostic.code)
                        .collect();
                    codes.sort();
                    let row = format!("{} at limit {limit}, warning {warning}:\n{text}", site.key);
                    if codes != expected {
                        failures.push(format!("{row}\n  draws {codes:?}, expected {expected:?}"));
                        continue;
                    }
                    if !warns {
                        continue;
                    }
                    let Some(drawn) = found
                        .iter()
                        .find(|diagnostic| diagnostic.code == Code::LimitWarning)
                    else {
                        continue;
                    };
                    let summary = format!(
                        "{}, the warning value of the limit of {limit}",
                        (site.counted)(warning)
                    );
                    let help = format!(
                        "reduce the work, or raise `{key}_warning` in the package's `options` or with `--limit {key}_warning=<value>`",
                        key = site.key
                    );
                    let twin = checked_within(
                        limit_and_warning(site.key, Some(warning), None),
                        &(site.program)(count),
                    );
                    let error = twin.iter().find(|diagnostic| diagnostic.code == site.error);
                    let placed = error.is_some_and(|error| {
                        let shift = beside.len() as u32;
                        drawn.span.range.start == error.span.range.start + shift
                            && drawn.span.range.end == error.span.range.end + shift
                            && drawn.related.len() == error.related.len()
                            && drawn
                                .related
                                .iter()
                                .zip(&error.related)
                                .all(|(ours, theirs)| {
                                    ours.label == theirs.label
                                        && ours.span.range.start == theirs.span.range.start + shift
                                })
                    });
                    if !drawn.summary.contains(&summary)
                        || drawn.help.as_deref() != Some(help.as_str())
                        || !placed
                    {
                        failures.push(format!("{row}\n  draws {drawn:?}\n  twin {error:?}"));
                    }
                }
            }
        }
    }
    (failures, taken)
}

#[test]
fn a_count_past_its_warning_value_draws_the_warning_where_the_limit_would_draw_its_error() {
    let (mut failures, _) = covered(|rows| {
        a_count_past_its_warning_value_draws_the_warning_where_the_limit_would_draw_its_error_in(
            rows,
        )
    });
    let each_site = (0..counted_sites().len()).flat_map(|site| {
        [
            [site, 0, 0, 0],
            [site, 0, 1, 0],
            [site, 0, 2, 0],
            [site, 1, 2, 0],
        ]
        .into_iter()
        .map(|tag| tag.to_vec())
    });
    let (each_failures, taken) =
        a_count_past_its_warning_value_draws_the_warning_where_the_limit_would_draw_its_error_in(
            &Rows::Chosen(std::sync::Mutex::new(each_site.collect())),
        );
    assert_eq!(taken, 36);
    failures.extend(each_failures);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_count_past_its_warning_value_draws_the_warning_where_the_limit_would_draw_its_error()
 {
    let (failures, taken) =
        a_count_past_its_warning_value_draws_the_warning_where_the_limit_would_draw_its_error_in(
            &Rows::Every,
        );
    assert_eq!(taken, 135);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "stress"]
fn stress_each_limit_draws_its_warning_past_half_its_default() {
    let rows = [
        (
            recursing(500_001),
            "this call of `g` exceeds 500,000 distinct recursive calls, the warning value of the limit of 1,000,000",
        ),
        (
            nested(513),
            "the evaluation goes deeper than 512 nested calls, the warning value of the limit of 1,024: r",
        ),
        (
            inlining(500_001),
            "makes the compilation inline more than 500,000 calls, the warning value of the limit of 1,000,000",
        ),
        (
            empty_iterations(500_001),
            "the expansion runs more than 500,000 `for` iterations and pattern uses, the warning value of the limit of 1,000,000",
        ),
        (
            iterations(500_000),
            "the expansion yields more than 500,000 instances and wires, the warning value of the limit of 1,000,000",
        ),
    ];
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    for (text, summary) in rows {
        let found: Vec<(Code, String)> = compiler
            .check(&[support::source(&text)])
            .into_iter()
            .filter(|diagnostic| diagnostic.code != Code::NoEntryInstance)
            .map(|diagnostic| (diagnostic.code, diagnostic.summary))
            .collect();
        assert!(
            matches!(found.as_slice(), [(Code::LimitWarning, drawn)] if drawn.ends_with(summary)),
            "{found:?} against {summary:?}"
        );
    }
}

#[test]
fn a_call_whose_inlining_passes_the_warning_value_and_the_limit_at_once_draws_the_error_alone() {
    let overrides = limit_and_warning("inlined_calls", Some(10), Some(5));
    for (text, expected) in [
        (inlining(7), vec![Code::LimitWarning]),
        (inlining(15), vec![Code::InlineLimit]),
        (inlining(14), vec![Code::LimitWarning, Code::InlineLimit]),
    ] {
        let codes: Vec<Code> = checked_within(overrides, &text)
            .iter()
            .filter(|diagnostic| diagnostic.code != Code::NoEntryInstance)
            .map(|diagnostic| diagnostic.code)
            .collect();
        assert_eq!(codes, expected, "{text}");
    }
}
