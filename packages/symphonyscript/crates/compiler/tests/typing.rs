mod support;

use symphonyscript::{Code, Compiler, Diagnostic, Source};
use symphonyscript_core::{Count, Vocabulary};
use symphonyscript_fixtures::{Rows, covered};

fn codes(text: &str) -> Vec<Code> {
    support::checked(text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn expect(cases: &[(&str, &[Code])]) {
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(text, expected)| {
            let found = codes(text);
            (found != *expected)
                .then(|| format!("{text:?}: expected {expected:?}, found {found:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_flexible_value_takes_the_type_of_each_use_and_must_fit_it() {
    expect(&[
        (
            "let base = 48\nClip m:\n    Note/(base + 7)\nSynth s:\n    Osc | freq=(base * 2.5)\n",
            &[],
        ),
        ("Clip c:\n    Note/-2147483648\n", &[]),
        ("Clip c:\n    Note/2147483648\n", &[Code::DoesNotFit]),
        ("Clip c:\n    Note/(2147483647 + 1)\n", &[Code::DoesNotFit]),
        ("Clip c:\n    Note | velocity=16777216\n", &[]),
        (
            "Clip c:\n    Note | velocity=16777217\n",
            &[Code::DoesNotFit],
        ),
        ("Clip c:\n    Note | velocity=3e38\n", &[]),
        ("Clip c:\n    Note | velocity=4e38\n", &[Code::DoesNotFit]),
        ("Synth s:\n    Osc | freq=4e38\n", &[]),
        (
            "let q = 2 ** -1\nSynth s:\n    Osc/q\nClip c:\n    Note/q\n",
            &[Code::InconsistentUses],
        ),
        (
            "let q = 2 ** 4\nSynth s:\n    Osc/q\nClip c:\n    Note/q\n",
            &[],
        ),
    ]);
}

#[test]
fn the_fit_check_applies_at_each_use_and_where_an_integer_converts_to_a_float() {
    expect(&[
        ("Clip c:\n    Note/(2147483647 * 2 div 2)\n", &[]),
        ("Clip c:\n    Note/(3000000000 - 2000000000)\n", &[]),
        ("Osc o | freq=(1.0 / (1e300 * 1e300))\n", &[]),
        ("Osc o | freq=((16777216 + 1) / 2)\n", &[]),
        ("Osc o | freq=(1e300 * 1e300)\n", &[Code::DoesNotFit]),
        ("Clip c | gain=(1.0 / 1e300)\n", &[Code::DoesNotFit]),
        ("Clip c | gain=(16777217 / 2)\n", &[Code::DoesNotFit]),
        ("Clip c | gain=(16777217i32 / 2)\n", &[]),
        (
            "state n(1.0)\nClip c | gain=(n / 16777217)\n",
            &[Code::DoesNotFit],
        ),
        ("state n(1.0)\nOsc o | freq=(n / 16777217)\n", &[]),
        ("Clip c | gain=(sqrt(16777217))\n", &[Code::DoesNotFit]),
        ("Osc o | freq=(sqrt(16777216 + 1))\n", &[]),
        ("Clip c | gain=(33554434 div 2)\n", &[Code::DoesNotFit]),
        ("Osc o | freq=(33554434 div 2)\n", &[]),
        (
            "Osc o | freq=170141183460469231731687303715884105727\n",
            &[Code::DoesNotFit],
        ),
    ]);
}

#[test]
fn a_float_type_that_flows_in_makes_a_flexible_value_a_float_from_its_literals_up() {
    expect(&[
        ("Clip c | gain=((16777216 + 1) / 2)\n", &[]),
        ("state n(1.0)\nClip c | gain=(n / (16777216 + 1))\n", &[]),
        ("Clip c | gain=(sqrt(16777216 + 1))\n", &[]),
        (
            "let c = (16777216 + 1) / 2 > 1\nClip k:\n    Note when c\n",
            &[Code::DoesNotFit],
        ),
        (
            "let c = (16777216 + 2) / 2 > 1\nClip k:\n    Note when c\n",
            &[],
        ),
        (
            "let c = sqrt(16777217) > 1\nClip k:\n    Note when c\n",
            &[Code::DoesNotFit],
        ),
    ]);
    for n in (1u32 << 24) - 3..=(1u32 << 24) + 9 {
        let exact = n <= 1 << 24 || n % 2 == 0;
        let expected: &[Code] = if exact { &[] } else { &[Code::DoesNotFit] };
        expect(&[
            (&format!("Clip c | gain=({n} / 2)\n"), expected),
            (&format!("Clip c | gain=(({n} - 1 + 1) / 2)\n"), expected),
            (&format!("Osc o | freq=({n} / 2)\n"), &[]),
            (
                &format!("let c = {n} / 2 > 1\nClip k:\n    Note when c\n"),
                expected,
            ),
        ]);
    }
}

#[test]
fn a_recursive_call_computes_flexible_integer_arguments_exactly_and_its_body_in_the_float_type_it_meets_292()
 {
    let fact = "fn fact(n): if n < 1 then 1 else n * fact(n - 1)\n";
    for k in 0u32..=30 {
        expect(&[(&format!("{fact}Osc o | freq=(fact({k}))\n"), &[])]);
    }
    expect(&[
        (
            "fn s(n): if n < 1 then 0 else (n div 2) + s(n - 1)\nOsc o | freq=(s(7))\n",
            &[],
        ),
        (
            "fn f(x): if x < 1 then x div 2 else f(x - 1)\nClip c:\n    Note/(f(0.5))\n",
            &[Code::OperandType],
        ),
    ]);
}

#[test]
fn every_guard_and_use_gate_is_checked_whether_or_not_it_gates_an_item() {
    expect(&[
        (
            "Clip c:\n    when 5:\n        for i in 0..0:\n            Note\n",
            &[Code::ConditionType],
        ),
        (
            "pattern P():\n    for i in 0..0:\n        Note\nClip c:\n    P when 5\n",
            &[Code::ConditionType],
        ),
        (
            "Clip c:\n    when:\n        true: Note\n        5: Note\n",
            &[Code::ConditionType],
        ),
        (
            "state fill_on(false)\nstate section(0)\nClip verse:\n    Note/60\n    when fill_on:\n        Note/62 Note/64\n    else:\n        Note/62\n    when:\n        section is 0: Note/67\n        section is 1: Note/69\n        else: Note/71\n",
            &[],
        ),
    ]);
}

#[test]
fn a_recursive_result_and_body_follow_the_unit_rules() {
    expect(&[
        (
            "fn r(n): if n < 1 then 2bars else r(n - 1)\nClip c:\n    Note/(r(3))\n",
            &[Code::UnitMismatch],
        ),
        (
            "fn r(n): if n < 1 then 2bars else r(n - 1)\nClip c:\n    Note | length=(r(3))\n",
            &[],
        ),
        (
            "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)\nClip c:\n    Note | length=(fib(3))\n",
            &[],
        ),
    ]);
    support::expect_codes(
        "fn t(n): if n < 1 then 0 else t(n - 1) + 1\nKit k:\n    Sampler | file=\"{t(2bars)}\"\n",
        &[(Code::PlainInFamily, "1", 0), (Code::PlainInFamily, "1", 1)],
    );
}

#[test]
fn a_flexible_integer_where_no_type_is_fixed_takes_the_defaults_and_fits_i64() {
    expect(&[
        (
            "let c = 10000000000000000000000 > 1\nClip k:\n    Note when c\n",
            &[Code::DoesNotFit],
        ),
        (
            "let c = 9223372036854775807 > 1\nClip k:\n    Note when c\n",
            &[],
        ),
        (
            "let c = -9223372036854775808 > 1\nClip k:\n    Note when c\n",
            &[],
        ),
        (
            "let c = 9223372036854775808 > 1\nClip k:\n    Note when c\n",
            &[Code::DoesNotFit],
        ),
        (
            "let c = 2 ** 70 div 2 ** 69 > 1\nClip k:\n    Note when c\n",
            &[],
        ),
        (
            "Kit k:\n    Sampler | file=\"{2 ** 100}\"\n",
            &[Code::DoesNotFit],
        ),
        ("Kit k:\n    Sampler | file=\"{2 ** 62}\"\n", &[]),
        ("Clip c:\n    Note/(i32(2 ** 100))\n", &[Code::DoesNotFit]),
        ("Clip c:\n    Note/(i32(3000000000))\n", &[]),
        (
            "let v = match 2 ** 70:\n    1: 1\n    else: 2\nClip c:\n    Note/v\n",
            &[Code::DoesNotFit],
        ),
        (
            "let v = match 2 ** 62:\n    1: 1\n    else: 2\nClip c:\n    Note/v\n",
            &[],
        ),
        ("Clip c:\n    Note/(2147483647 * 2 div 2)\n", &[]),
        ("Clip c:\n    for i in 0..2:\n        Note/i\n", &[]),
    ]);
}

#[test]
fn a_flexible_argument_of_rand_takes_i64_and_fits_it() {
    expect(&[
        ("Clip c | gain=(rand(2 ** 70, 1))\n", &[Code::DoesNotFit]),
        (
            "Clip c | gain=(rand(1, -(2 ** 63) - 1))\n",
            &[Code::DoesNotFit],
        ),
        ("Clip c | gain=(rand(2 ** 63 - 1, -(2 ** 63)))\n", &[]),
    ]);
}

#[test]
fn a_literal_or_use_a_fixed_type_operand_types_fits_that_type() {
    support::expect_codes(
        "Clip c:\n    Note/(5i32 + 3000000000)\n",
        &[(Code::DoesNotFit, "3000000000", 0)],
    );
    support::expect_codes(
        "let x = 3000000000\nClip c:\n    Note/(5i32 + x)\n",
        &[(Code::DoesNotFit, "x", 1)],
    );
    expect(&[
        ("let x = 5\nClip c:\n    Note/(5i32 + x)\n", &[]),
        (
            "Clip c:\n    Note/(2147483647i32 + 1)\n",
            &[Code::ConstantOverflow],
        ),
        ("Clip c:\n    Note/(5i32 + 2147483642)\n", &[]),
        (
            "Clip c:\n    Note/(5i32 + 2147483643)\n",
            &[Code::ConstantOverflow],
        ),
        (
            "let x = 2147483647i32 + 1\nClip c:\n    Note/x\n",
            &[Code::ConstantOverflow],
        ),
    ]);
}

#[test]
fn a_nan_at_a_use_does_not_fit() {
    expect(&[
        ("Osc o | freq=(0.0 / 0.0)\n", &[Code::DoesNotFit]),
        (
            "let c = (0.0 / 0.0) > 1\nClip k:\n    Note when c\n",
            &[Code::DoesNotFit],
        ),
        ("Osc o | freq=(max(0.0 / 0.0, 440.0))\n", &[]),
    ]);
}

#[test]
fn the_two_to_the_127_bound_counts_magnitude() {
    expect(&[
        (
            "Clip c:\n    Note/(-(2 ** 126) * 2 div 2 ** 120)\n",
            &[Code::IntegerOverflow],
        ),
        (
            "Clip c:\n    Note/((-(2 ** 126) - (2 ** 126 - 1)) div 2 ** 120)\n",
            &[],
        ),
        (
            "Clip c:\n    Note/((-2) ** 127 div 2 ** 120)\n",
            &[Code::IntegerOverflow],
        ),
        (
            "Clip c:\n    Note/((-(2 ** 126) - 2 ** 126) div 2 ** 120)\n",
            &[Code::IntegerOverflow],
        ),
        (
            "Clip c:\n    Note/((2 ** 126 + (2 ** 126 - 1)) div 2 ** 120)\n",
            &[],
        ),
    ]);
}

#[test]
fn a_comparison_that_reads_an_integer_input_as_a_float_is_a_float_use() {
    let integer_inputs = |text: &str| {
        support::checked_all(text)
            .iter()
            .filter(|diagnostic| diagnostic.code == Code::IntegerCell)
            .count()
    };
    assert_eq!(
        integer_inputs("state n(4)\nClip c:\n    Note when n > 0.5\n"),
        1
    );
    assert_eq!(
        integer_inputs("state n(4)\nClip c:\n    Note when n > 1\n"),
        0
    );
    assert_eq!(
        integer_inputs("state n(4)\nClip c:\n    Note when n + 1 > 0.5\n"),
        1
    );
}

#[test]
fn flexible_integers_compute_exactly_below_two_to_the_127() {
    for k in 31..=126 {
        expect(&[
            (
                &format!("Clip c:\n    Note/(2 ** {k} div 2 ** ({k} - 10))\n"),
                &[],
            ),
            (
                &format!("Clip c:\n    Note/(2 ** {k})\n"),
                &[Code::DoesNotFit],
            ),
        ]);
    }
    expect(&[(
        "Clip c:\n    Note/(2 ** 127 div 2 ** 117)\n",
        &[Code::IntegerOverflow],
    )]);
}

#[test]
fn an_unused_compile_time_let_is_never_computed() {
    expect(&[
        ("let e = 2 ** 2 ** 2 ** 2 ** 2\n", &[]),
        ("let x = [1, 2][5]\n", &[]),
        ("let y = 1 + true\n", &[]),
        (
            "let y = 1 + true\nClip c:\n    Note/y\n",
            &[Code::OperandType],
        ),
        (
            "let e = 2 ** 2 ** 2 ** 2 ** 2\nClip c:\n    Note/e\n",
            &[Code::IntegerOverflow],
        ),
        (
            "let x = [1, 2][5]\nClip c:\n    Note/x\n",
            &[Code::IndexOutOfRange],
        ),
    ]);
}

#[test]
fn an_expected_type_flows_into_operands_and_fixed_types_only_widen() {
    expect(&[
        (
            "state cutoff(0.5)\nClip m:\n    Note | brightness=cutoff * 0.8\n",
            &[],
        ),
        ("state n(4)\nClip c:\n    Note/(n div 2)\n", &[]),
        (
            "state n(4)\nClip c:\n    Note/(n / 2)\n",
            &[Code::TypeMismatch],
        ),
        ("Synth s:\n    Osc | freq=1.0f32\n", &[]),
        (
            "Clip c:\n    Note | velocity=1.0f64\n",
            &[Code::TypeMismatch],
        ),
        (
            "Clip c:\n    Note | pitch=(1i32 + 2i64)\n",
            &[Code::TypeMismatch],
        ),
        ("Synth s:\n    Osc | freq=(1i32 + 2i64)\n", &[]),
        ("Clip c:\n    Note | velocity=(1i32 + 0.5)\n", &[]),
        (
            "Clip c:\n    Note | velocity=(1i64 + 0.5)\n",
            &[Code::TypeMismatch],
        ),
        ("Clip c:\n    Note | velocity=(1 / 4)\n", &[]),
        ("Clip c:\n    Note/(i32(3000000000))\n", &[]),
    ]);
}

#[test]
fn a_cell_is_read_in_each_use_type_and_its_initial_value_fits_every_use() {
    expect(&[
        ("state a(3000000000)\nSynth s:\n    Osc | freq=a\n", &[]),
        (
            "state a(3000000000)\nClip c:\n    Note/a\n",
            &[Code::DoesNotFit],
        ),
        (
            "state fine(0): f32\nClip c:\n    Note | velocity=fine\n",
            &[],
        ),
        ("state a(0.5): i32\n", &[Code::TypeMismatch]),
        (
            "state a(1)\nlet b = a + 0.5\nClip c:\n    Note when b > 1\n",
            &[],
        ),
        (
            "state f(0.5)\nlet g = f * 2\nClip c:\n    Note/g\n",
            &[Code::FloatToInteger],
        ),
        ("state f(0.5)\nClip c:\n    Note/(i32(f))\n", &[]),
    ]);
}

#[test]
fn a_reactive_let_read_by_another_keeps_the_width_of_the_input_it_reads() {
    for links in 1..=50 {
        for (annotation, misfits) in [("", 0), (": i32", links)] {
            let mut text = format!("state a(1){annotation}\nlet l1 = a + 1\n");
            for link in 2..=links {
                text.push_str(&format!("let l{link} = l{} * 2\n", link - 1));
            }
            text.push_str("Clip k:\n");
            for link in 1..=links {
                text.push_str(&format!("    Note when l{link} > 3000000000\n"));
            }
            let expected = vec![Code::DoesNotFit; misfits];
            assert_eq!(codes(&text), expected, "{text}");
        }
    }
}

#[test]
fn an_annotation_fixes_a_type_outright() {
    expect(&[
        (
            "let x: i32 = 0.5\nClip c:\n    Note/x\n",
            &[Code::TypeMismatch],
        ),
        ("let x: f32 = 1\nSynth s:\n    Osc | freq=x\n", &[]),
        (
            "let x: f64 = 1\nClip c:\n    Note | velocity=x\n",
            &[Code::TypeMismatch],
        ),
        (
            "let x = 2147483647i32 + 1\nClip c:\n    Note/x\n",
            &[Code::ConstantOverflow],
        ),
    ]);
}

#[test]
fn only_the_chosen_branch_of_a_compile_time_choice_is_checked_for_fit_and_index_range() {
    expect(&[
        (
            "Clip c:\n    Note/(if true then [1, 2][0] else [1, 2][5])\n",
            &[],
        ),
        (
            "Clip c:\n    Note/(if false then [1, 2][0] else [1, 2][5])\n",
            &[Code::IndexOutOfRange],
        ),
        ("Clip c:\n    Note/(if true then 1 else 3000000000)\n", &[]),
        (
            "Clip c:\n    Note/(if false then 1 else 3000000000)\n",
            &[Code::DoesNotFit],
        ),
        (
            "state b(true)\nClip c:\n    Note/(if b then 1 else 3000000000)\n",
            &[Code::DoesNotFit],
        ),
        ("state i(0)\nClip c:\n    Note/([1, 2][i + 5])\n", &[]),
    ]);
}

#[test]
fn enums_strings_and_bools_take_their_own_types() {
    expect(&[
        ("Synth s:\n    Osc | wave=Saw\n", &[]),
        ("state w(Wave::Saw)\nSynth s:\n    Osc | wave=w\n", &[]),
        ("Synth s:\n    Osc | wave=1\n", &[Code::TypeMismatch]),
        ("let w = Saw\nSynth s:\n    Osc | wave=w\n", &[]),
        (
            "Kit k:\n    for i in 0..2:\n        Sampler | file=\"kit/{i}.wav\"\n",
            &[],
        ),
        ("Kit k:\n    Sampler | file=1\n", &[Code::TypeMismatch]),
        ("state c(0.5)\nClip m:\n    Note | accent=(c > 0.5)\n", &[]),
    ]);
}

#[test]
fn a_bare_variant_where_no_enum_is_expected_draws_unknown_name_wherever_it_stands() {
    let saw = (Code::UnknownName, "Saw");
    let rows: &[support::Row] = &[
        (&["Clip c:\n    Note/(60 + [Saw].size)\n"], &[saw]),
        (
            &["Clip c:\n    Note/(60 + [Saw, Square].size)\n"],
            &[saw, (Code::UnknownName, "Square")],
        ),
        (
            &["let a = [Saw]\n", "Clip c:\n    Note/(60 + a.size)\n"],
            &[(Code::TypeMismatch, "[Saw]")],
        ),
        (
            &[
                "state w(Wave::Saw)\n",
                "Clip c:\n    Note/(60 + [w].size)\n",
            ],
            &[],
        ),
        (&["Synth s:\n    Osc | wave=Saw\n"], &[]),
        (
            &["Synth s:\n    for w in [Saw, Square]:\n        Osc | wave=w\n"],
            &[saw, (Code::UnknownName, "Square")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let text = "Clip c:\n    Note/(60 + [Saw].size)\n";
    let found = support::errors(compiler.check(&[support::source(text)]));
    let helps: Vec<Option<&str>> = found
        .iter()
        .map(|diagnostic| diagnostic.help.as_deref())
        .collect();
    assert_eq!(
        helps,
        [Some(
            "write `Wave::Saw`, or use `Saw` in a position that expects a `Wave`"
        )],
        "{text}"
    );
}

#[test]
fn a_bare_variant_stands_only_where_an_enum_is_expected() {
    expect(&[
        ("Clip c | gain=Saw(1)\n", &[Code::UnknownName]),
        ("let z = Saw.gain\nClip c | gain=z\n", &[Code::UnknownName]),
        (
            "state w(Saw)\nSynth s:\n    Osc | wave=w\n",
            &[Code::UnknownName],
        ),
        (
            "Synth s:\n    for v in [Saw, Sine]:\n        Osc | wave=v\n",
            &[Code::UnknownName, Code::UnknownName],
        ),
        (
            "fn r(n, w): if n < 1 then w else r(n - 1, w)\nSynth s:\n    Osc | wave=(r(2, Saw))\n",
            &[Code::UnknownName],
        ),
        ("Synth s:\n    Osc | wave=Saw\n", &[]),
        ("state w(Wave::Saw)\nSynth s:\n    Osc when w is Saw\n", &[]),
        (
            "state w(Wave::Saw)\nlet level = match w:\n    Sine: 1.0\n    Saw: 0.8\n    else: 0.6\nClip c | gain=level\n",
            &[],
        ),
        ("let w = Saw\nSynth s:\n    Osc | wave=w\n", &[]),
        (
            "Synth s:\n    for v in [Wave::Saw, Wave::Sine]:\n        Osc | wave=v\n",
            &[],
        ),
    ]);
}

#[test]
fn the_operands_of_is_give_a_reactive_let_its_enum() {
    let held = "Session s\nstate f(true)\nlet x = if f then Sine else Saw\n";
    let rows = [
        format!("{held}Clip c:\n    Note when x is Wave::Saw\n"),
        format!("{held}Clip c:\n    Note when x is not Wave::Saw\n"),
        format!("{held}Clip c:\n    Note when Wave::Saw is x\n"),
        format!("{held}state w(Wave::Saw)\nClip c:\n    Note when x is w\n"),
        format!("{held}let g = x is Wave::Saw\nClip c:\n    Note when g\n"),
        format!(
            "{held}let y = if f then Saw else Sine\nOsc o | wave=x\nClip c:\n    Note when x is y\n"
        ),
        format!("{held}Osc o | wave=x\nClip c:\n    Note when x is Saw\n"),
        format!("{held}Osc o | wave=x\n"),
        "let x = Sine\nClip c:\n    Note when x is Wave::Saw\n".into(),
    ];
    let cases: Vec<(&str, &[Code])> = rows.iter().map(|text| (text.as_str(), &[][..])).collect();
    expect(&cases);
}

#[test]
fn a_let_an_error_leaves_untyped_reads_as_a_placeholder_at_its_uses() {
    let held = "Session s\nstate f(true)\nlet x = if f then Sine else Saw\n";
    let the_value = "if f then Sine else Saw";
    for uses in [
        "Clip c:\n    Note when x is x\n",
        "Clip c:\n    Note when x is Saw\n",
        "Clip c:\n    Note/x\n",
        "Clip c:\n    Note/(x + 1)\n",
    ] {
        support::expect_codes(
            &format!("{held}{uses}"),
            &[(Code::TypeMismatch, the_value, 0)],
        );
    }
    support::expect_codes(
        "state f(true)\nlet x = if f then \"a\" else \"b\"\nClip c:\n    Note when x is \"a\"\n",
        &[(
            Code::ReactiveString,
            "let x = if f then \"a\" else \"b\"\n",
            0,
        )],
    );
    support::expect_codes(
        &format!("{held}let y = x\nClip c:\n    Note when y is 1\n"),
        &[(Code::TypeMismatch, the_value, 0)],
    );
    let named = "let t = if x is Wave::Saw then \"a\" else \"b\"\n";
    support::expect_codes(
        &format!("{held}{named}Kit k:\n    Sampler | file=t\n"),
        &[(Code::ReactiveString, named, 0)],
    );
    let typed = format!("{held}Osc o | wave=x\n");
    support::expect_codes(
        &format!("{typed}Clip c:\n    Note/(x + 1)\n"),
        &[(Code::OperandType, "x + 1", 0)],
    );
    support::expect_codes(
        &format!("{typed}Clip c:\n    Note when x is 1\n"),
        &[(Code::OperandType, "x is 1", 0)],
    );
    support::expect_codes(
        "Clip c:\n    Note when 1 is true\n",
        &[(Code::OperandType, "1 is true", 0)],
    );
}

#[test]
fn a_bare_variant_lets_diagnostic_stands_where_its_variants_are_written() {
    let held = "Session s\nstate f(true)\nstate g(true)\nlet x = if f then Sine else Saw\n";
    let other = "let w = if f then Saw else Square\n";
    let bare = "Session s\nstate f(true)\nstate g(true)\n";
    let the_value = "if f then Sine else Saw";
    let other_value = "if f then Saw else Square";
    let failures: Vec<String> = [
        (
            held,
            "let y = x\n",
            "    Note when y + true\n",
            vec![(the_value, vec!["let y"])],
        ),
        (
            held,
            "let y = x\nlet z = y\n",
            "    Note when z + true\n",
            vec![(the_value, vec!["let y", "let z"])],
        ),
        (
            held,
            "let y = x\n",
            "    Note when x + true\n    Note when y + true\n",
            vec![(the_value, vec!["let y"])],
        ),
        (
            held,
            "let y = x\nlet z = x\n",
            "    Note when y + true\n    Note when z + true\n",
            vec![(the_value, vec!["let y", "let z"])],
        ),
        (
            held,
            "let w = if f then Saw else Square\nlet y = if g then x else w\n",
            "    Note when y + true\n",
            vec![(the_value, vec![other, "let y"])],
        ),
        (
            held,
            "let y = if f then x else Square\n",
            "    Note when y + true\n",
            vec![(the_value, vec!["let y"])],
        ),
        (
            held,
            "let w = if f then Saw else Square\n",
            "    Note when x is w\n",
            vec![(the_value, vec![other])],
        ),
        (
            held,
            "let w = if f then Saw else Square\n",
            "    Note when x + true\n    Note when w + true\n",
            vec![(the_value, vec![]), (other_value, vec![])],
        ),
        (
            held,
            "",
            "    Note when x + true\n",
            vec![(the_value, vec![])],
        ),
        (
            bare,
            "let v = w\nlet w = if f then Saw else Sine\n",
            "    Note/60\n",
            vec![("if f then Saw else Sine", vec!["let v"])],
        ),
        (
            bare,
            "let u = if f then v else v\nlet v = w\nlet w = if f then Saw else Sine\n",
            "    Note/60\n",
            vec![("if f then Saw else Sine", vec!["let u", "let v"])],
        ),
    ]
    .into_iter()
    .filter_map(|(prefix, lets, uses, expected)| {
        let text = format!("{prefix}{lets}Clip c:\n{uses}");
        let start = |needle: &str| support::at(&text, needle, 0).0;
        let expected: Vec<(Code, u32, Vec<u32>)> = expected
            .iter()
            .map(|(primary, secondary)| {
                let secondary = secondary.iter().map(|needle| start(needle)).collect();
                (Code::TypeMismatch, start(primary), secondary)
            })
            .collect();
        let found: Vec<(Code, u32, Vec<u32>)> = support::checked(&text)
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code,
                    diagnostic.span.range.start,
                    diagnostic
                        .related
                        .iter()
                        .map(|related| &related.span)
                        .map(|span| span.range.start)
                        .collect(),
                )
            })
            .collect();
        (found != expected).then(|| format!("{text}expected {expected:?}, found {found:?}"))
    })
    .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_let_that_reads_a_typed_enum_takes_its_type() {
    let held = "Session s\nstate f(true)\nlet x = if f then Sine else Saw\n";
    let typed = "Synth s2:\n    Osc | wave=x\n";
    support::expect_codes(
        "Session s\nstate w(Wave::Saw)\nlet y = w\nClip c:\n    Note when y\n",
        &[(Code::ConditionType, "y", 1)],
    );
    support::expect_codes(
        &format!("{held}let y = x\nClip c:\n    Note when y\n{typed}"),
        &[(Code::ConditionType, "y", 1)],
    );
    support::expect_codes(
        &format!("{held}let y = x\nlet z = if f then y else x\nClip c:\n    Note when z\n{typed}"),
        &[(Code::ConditionType, "z", 1)],
    );
    support::expect_codes(
        &format!("{held}let y = x\nClip c:\n    Note when y is Saw\n{typed}"),
        &[],
    );
    for (lets, held) in [
        ("let y = if f then x else Square\n", held),
        ("state i(0)\nlet y = [x, Square][i]\n", held),
        (
            "let y = if f then w else Square\n",
            "Session s\nstate f(true)\nstate w(Wave::Saw)\nlet x = if f then Sine else Saw\n",
        ),
    ] {
        support::expect_codes(
            &format!("{held}{lets}Clip c:\n    Note when y\n{typed}"),
            &[(Code::ConditionType, "y", 1)],
        );
    }
    support::expect_codes(
        &format!("{held}let y = x\nClip c:\n    Note when y + true\n"),
        &[(Code::TypeMismatch, "if f then Sine else Saw", 0)],
    );
}

fn two_enum_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("enums");
    let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);
    let mode = v.enumeration("Mode", ["Low", "High"]);
    let color = v.enumeration("Color", ["Red", "Green", "Blue"]);
    let osc = v.node("Osc");
    let filter = v.node("Filt");
    let both = v.node("K");
    v.kind(osc)
        .top_level(Count::Any)
        .attr_enum("wave", &wave, "Sine")
        .attr::<f64>("freq")
        .default(440.0);
    v.kind(filter)
        .top_level(Count::Any)
        .attr_enum("mode", &mode, "Low");
    v.kind(both)
        .top_level(Count::Any)
        .attr_enum("wave", &wave, "Sine")
        .attr_enum("color", &color, "Red");
    v.build().expect("the two-enum vocabulary builds")
}

type Errors = Vec<(Code, String)>;

fn a_carried_enum_meets_every_use_in_every_order_in(orders: support::Orders) {
    let compiler = Compiler::new(&two_enum_vocabulary()).expect("the vocabulary compiles");
    let errors_in = |text: &str| -> Errors {
        let source = Source {
            path: "test.sym".into(),
            text: text.into(),
        };
        let mut found: Errors = support::errors(compiler.check(&[source]))
            .iter()
            .map(|diagnostic| {
                let range = diagnostic.span.range.clone();
                let at = usize::try_from(range.start).unwrap_or(0)
                    ..usize::try_from(range.end).unwrap_or(0);
                (diagnostic.code, text.get(at).unwrap_or("").to_string())
            })
            .collect();
        found.sort();
        found
    };
    let carried = [
        "state f(true)",
        "let w = if f then Saw else Sine",
        "let v = if f then w else w",
    ];
    let held = [
        "state f(true)",
        "let a = if f then Sine else Saw",
        "let b = a",
    ];
    let with = |base: &[&'static str], items: &[&'static str]| -> Vec<&'static str> {
        base.iter().chain(items).copied().collect()
    };
    let rejected = [
        with(&carried, &["Osc o | wave=w", "Filt g | mode=v"]),
        with(&carried, &["Osc p | wave=v", "Filt g | mode=v"]),
        with(
            &carried,
            &[
                "let u = if f then v else v",
                "Osc o | wave=w",
                "Filt g | mode=u",
            ],
        ),
        with(
            &carried,
            &["Osc o | wave=w freq=(if v is Mode::Low then 1 else 2)"],
        ),
        with(&held, &["K k1 | wave=a color=b"]),
        with(&held, &["K k1 | wave=a", "K k2 | color=b", "K k3 | wave=b"]),
        with(&held, &["let c = b", "K k1 | wave=a color=c"]),
    ];
    let joined = [
        "state f(true)",
        "let a = if f then Sine else Saw",
        "let b = if f then Red else Green",
        "let y = if f then a else b",
        "K k1 | wave=a color=b",
    ];
    let colored = "let b = match a:\n    Red: Blue\n    Blue: Red\n    else: Green";
    let accepted = [
        vec!["let a = Red", colored, "K k1 | color=a", "K k2 | color=b"],
        vec![
            "state f(true)",
            "let a = if f then Red else Blue",
            colored,
            "K k1 | color=a",
            "K k2 | color=b",
        ],
        with(&carried, &["Osc p | wave=v"]),
        with(
            &carried,
            &["Osc p | wave=v freq=(if v is Saw then 1 else 2)"],
        ),
        with(&carried, &["Osc o | wave=w", "Osc p | wave=v"]),
        with(&held, &["K k1 | wave=a", "K k2 | wave=b"]),
    ];
    let mut failures = Vec::new();
    let mut cases: Vec<(Vec<&'static str>, Option<Errors>)> =
        rejected.into_iter().map(|items| (items, None)).collect();
    cases.push((
        vec![
            "state f(true)",
            "let a = if f then Sine else Saw",
            "let b = if f then a else b",
            "K k1 | wave=b",
        ],
        Some(vec![(Code::DeclarationCycle, "b".to_string())]),
    ));
    cases.push((
        joined.to_vec(),
        Some(vec![(Code::TypeMismatch, "if f then a else b".to_string())]),
    ));
    cases.extend(accepted.into_iter().map(|items| (items, Some(Vec::new()))));
    for (items, expected) in cases {
        let mut first: Option<Errors> = None;
        for order in orders.of(&items) {
            let text = format!("{}\n", order.join("\n"));
            let found = errors_in(&text);
            let wrong = match &expected {
                Some(expected) => found != *expected,
                None => found.is_empty() || first.as_ref().is_some_and(|first| *first != found),
            };
            if wrong {
                failures.push(format!("{text}found {found:?}"));
                break;
            }
            first.get_or_insert(found);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_carried_enum_meets_every_use_forward_and_reversed() {
    a_carried_enum_meets_every_use_in_every_order_in(support::Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_carried_enum_meets_every_use_in_every_order() {
    a_carried_enum_meets_every_use_in_every_order_in(support::Orders::Every);
}

#[test]
fn unit_values_keep_their_family() {
    expect(&[
        ("Clip c:\n    Note | length=2bars + 1\n", &[]),
        ("Clip c:\n    Note | length=200ms\n", &[Code::UnitMismatch]),
        (
            "let d = 5ms\nClip c:\n    Note when d > 1\n",
            &[Code::PlainInFamily],
        ),
        (
            "state d(5ms)\nlet e = d * 2\nClip c:\n    Note | length=e\n",
            &[Code::UnitMismatch],
        ),
        ("Clip c:\n    Note | length=(2bars / 1bars)\n", &[]),
        ("Synth s:\n    Osc | freq=(1s / 1ms)\n", &[]),
    ]);
}

#[test]
fn a_match_waits_for_its_scrutinees_enum() {
    let arms = "let l = match a:\n    Sine: 1.0\n    else: 0.5\n";
    let used = "Synth y:\n    Osc | wave=a rate=l\n";
    for held in [
        "Session s\nlet a = Sine\n",
        "Session s\nstate a(Wave::Saw)\n",
        "Session s\nstate f(true)\nlet a = if f then Sine else Saw\n",
    ] {
        support::expect_codes(&format!("{held}{arms}{used}"), &[]);
    }
    let unused = "Synth y:\n    Osc | rate=l\n";
    support::expect_codes(
        &format!("Session s\nstate f(true)\nlet a = if f then Sine else Saw\n{arms}{unused}"),
        &[(Code::TypeMismatch, "if f then Sine else Saw", 0)],
    );
    support::expect_codes(
        &format!("Session s\nlet a = Sine\n{arms}{unused}"),
        &[(Code::TypeMismatch, "Sine", 0)],
    );
    support::expect_codes(
        &format!("Session s\nlet a = Sine\nlet l = match a:\n    Low: 1.0\n    else: 0.5\n{used}"),
        &[(Code::MatchLabel, "Low", 0)],
    );
    support::expect_codes(
        &format!("Session s\nlet a = Sine\nlet l = match a:\n    Sine: 1.0\n    Saw: 0.5\n{used}"),
        &[(
            Code::MatchIncomplete,
            "match a:\n    Sine: 1.0\n    Saw: 0.5",
            0,
        )],
    );
}

fn fixture_errors(text: &str) -> Errors {
    let mut found: Errors = support::checked(text)
        .iter()
        .map(|diagnostic| {
            let range = diagnostic.span.range.clone();
            let at =
                usize::try_from(range.start).unwrap_or(0)..usize::try_from(range.end).unwrap_or(0);
            (diagnostic.code, text.get(at).unwrap_or("").to_string())
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_compile_time_bare_let_typed_by_its_uses_is_a_typed_value_for_every_consumer() {
    let typed = "Synth s:\n    Osc | wave=x";
    let consumers: [&[&'static str]; 8] = [
        &["state k(x)"],
        &["Clip c:\n    for e in [x]:\n        Note/60"],
        &["let y = x", "state k(y)"],
        &["Track t | label=\"{x}\""],
        &["Osc p when x is Saw"],
        &["let y = Saw", "Osc p when x is y"],
        &[
            "state f(true)",
            "let v = if f then x else Saw",
            "Osc p when v is Saw",
        ],
        &[
            "state f(true)",
            "let v = if f then x else x",
            "Osc p when v is Saw",
        ],
    ];
    let mut failures = Vec::new();
    for consumer in consumers {
        for written in ["let x = Sine", "let x = Wave::Sine"] {
            let items: Vec<&'static str> = std::iter::once(written)
                .chain(consumer.iter().copied())
                .chain(std::iter::once(typed))
                .collect();
            for order in support::permutations(&items) {
                let text = format!("{}\n", order.join("\n"));
                let found = fixture_errors(&text);
                if !found.is_empty() {
                    failures.push(format!("{text}found {found:?}"));
                    break;
                }
            }
        }
    }
    let untyped: [&[&'static str]; 4] = [
        &["state k(x)"],
        &["Clip c:\n    for e in [x]:\n        Note/60"],
        &["Track t | label=\"{x}\""],
        &["Clip c:\n    Note when x"],
    ];
    for consumer in untyped {
        let items: Vec<&'static str> = std::iter::once("let x = Sine")
            .chain(consumer.iter().copied())
            .collect();
        for order in support::permutations(&items) {
            let text = format!("{}\n", order.join("\n"));
            let found = fixture_errors(&text);
            if found != [(Code::TypeMismatch, "Sine".to_string())] {
                failures.push(format!("{text}found {found:?}"));
                break;
            }
        }
    }
    let compared = ["let x = Sine", "let y = Saw", "Osc p when x is y"];
    for order in support::permutations(&compared) {
        let text = format!("{}\n", order.join("\n"));
        let first = order
            .iter()
            .find(|item| item.starts_with("let"))
            .and_then(|item| item.split_once(" = "))
            .map(|(_, value)| value.to_string())
            .unwrap_or_default();
        let found = fixture_errors(&text);
        if found != [(Code::TypeMismatch, first)] {
            failures.push(format!("{text}found {found:?}"));
            break;
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn an_array_literals_expected_type_reaches_every_element() {
    let mut failures = Vec::new();
    for (written, items) in [
        (
            "let x = Sine",
            vec!["Synth y:\n    Osc | wave=([x, Wave::Square][1])"],
        ),
        (
            "let x = Sine",
            vec!["Synth y:\n    Osc | wave=([x, Square][1])"],
        ),
        (
            "let x = Sine",
            vec!["let z = [x, Square][1]", "Synth y:\n    Osc | wave=z"],
        ),
        (
            "let x = Sine",
            vec!["Synth y:\n    Osc | wave=([x, Wave::Square][0])"],
        ),
        (
            "let x = if f then Sine else Saw",
            vec![
                "state f(true)",
                "Synth y:\n    Osc | wave=([x, Wave::Square][1])",
            ],
        ),
    ] {
        let all: Vec<&'static str> = ["Session s", written].into_iter().chain(items).collect();
        for order in support::permutations(&all) {
            let text = format!("{}\n", order.join("\n"));
            let found = fixture_errors(&text);
            if !found.is_empty() {
                failures.push(format!("{text}found {found:?}"));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

type Row = (&'static [&'static str], &'static [(Code, &'static str)]);

#[test]
fn a_compile_time_choice_type_checks_what_it_does_not_select() {
    let enums = Compiler::new(&two_enum_vocabulary()).expect("the vocabulary compiles");
    let enum_errors = |text: &str| -> Errors {
        let source = Source {
            path: "test.sym".into(),
            text: text.into(),
        };
        let mut found: Errors = support::errors(enums.check(&[source]))
            .iter()
            .map(|diagnostic| {
                let range = diagnostic.span.range.clone();
                let at = usize::try_from(range.start).unwrap_or(0)
                    ..usize::try_from(range.end).unwrap_or(0);
                (diagnostic.code, text.get(at).unwrap_or("").to_string())
            })
            .collect();
        found.sort();
        found
    };
    let fixture_rows: [Row; 11] = [
        (
            &["Clip c:\n    Note/([60, \"s\"][0])"],
            &[(Code::TypeMismatch, "[60, \"s\"]")],
        ),
        (
            &["let a = [1, \"s\"]", "Clip c:\n    Note/(a[0])"],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &[
                "let g = [[1, 2], [3, \"s\"]]",
                "Clip c:\n    Note/(g[0][1])",
            ],
            &[(Code::TypeMismatch, "[3, \"s\"]")],
        ),
        (
            &["Clip c:\n    Note/([[1, 2], [3, \"s\"]][0][0])"],
            &[(Code::TypeMismatch, "[3, \"s\"]")],
        ),
        (
            &["Clip c:\n    Note/([1, 2.5][0])"],
            &[(Code::TypeMismatch, "[1, 2.5][0]")],
        ),
        (
            &["let a = [1, 2.5]", "Clip c:\n    Note/(a[0])"],
            &[(Code::TypeMismatch, "a[0]")],
        ),
        (
            &["Clip c:\n    Note/([1, 2.5][0] + 1)"],
            &[(Code::TypeMismatch, "[1, 2.5][0] + 1")],
        ),
        (
            &["Clip c:\n    Note/([1, 2i64][0])"],
            &[(Code::TypeMismatch, "[1, 2i64][0]")],
        ),
        (&["Clip c:\n    Note | pitch=([1, 2][0])"], &[]),
        (
            &[
                "state a(1)",
                "state b(3000000000)",
                "let c = [a, b]",
                "Clip m:\n    Note/(c[0])",
            ],
            &[],
        ),
        (
            &["let a = [1, 3000000000]", "Clip c:\n    Note/(a[0])"],
            &[],
        ),
    ];
    let enum_rows: [Row; 13] = [
        (
            &["Osc o | wave=([Sine, Red][0])"],
            &[(Code::UnknownName, "Red")],
        ),
        (
            &["Osc o | wave=([Sine, Color::Red][0])"],
            &[(Code::TypeMismatch, "[Sine, Color::Red][0]")],
        ),
        (
            &["Osc o | wave=([Sine, 3][0])"],
            &[(Code::TypeMismatch, "[Sine, 3]")],
        ),
        (
            &["let a = [Sine, Red]", "Osc o | wave=(a[0])"],
            &[(Code::UnknownName, "Red")],
        ),
        (
            &["let x = Sine", "let z = [x, Red][0]", "Osc o | wave=z"],
            &[(Code::UnknownName, "Red")],
        ),
        (
            &["Osc o | wave=(if true then Sine else Red)"],
            &[(Code::UnknownName, "Red")],
        ),
        (
            &["Osc o | wave=(if false then Red else Sine)"],
            &[(Code::UnknownName, "Red")],
        ),
        (
            &[
                "state w(Wave::Saw)",
                "Osc o | wave=(if true then w else Red)",
            ],
            &[(Code::UnknownName, "Red")],
        ),
        (
            &["Osc o | freq=(if true then 1.0 else (if Wave::Sine is Red then 1.0 else 2.0))"],
            &[(Code::UnknownName, "Red")],
        ),
        (
            &["Osc o | freq=([1.0, (if Wave::Sine is Red then 1.0 else 2.0)][0])"],
            &[(Code::UnknownName, "Red")],
        ),
        (
            &[
                "let q = match true:\n    true: Sine\n    false: Red",
                "Osc o | wave=q",
            ],
            &[(Code::UnknownName, "Red")],
        ),
        (&["Osc o | wave=(if true then Sine else Saw)"], &[]),
        (
            &["Osc o | freq=(if true then 1.0 else (if Wave::Sine is Saw then 1.0 else 2.0))"],
            &[],
        ),
    ];
    let mut failures = Vec::new();
    for (vocabulary, rows) in [("fixture", &fixture_rows[..]), ("enums", &enum_rows[..])] {
        for (items, wanted) in rows.iter() {
            let wanted: Errors = wanted
                .iter()
                .map(|(code, span)| (*code, (*span).to_string()))
                .collect();
            let mut all: Vec<&'static str> = items.to_vec();
            if vocabulary == "fixture" {
                all.insert(0, "Session s");
            }
            for order in support::permutations(&all) {
                let text = format!("{}\n", order.join("\n"));
                let found = if vocabulary == "fixture" {
                    fixture_errors(&text)
                } else {
                    enum_errors(&text)
                };
                if found != wanted {
                    failures.push(format!("{text}expected {wanted:?}\nfound {found:?}"));
                    break;
                }
            }
        }
    }
    let passed_over =
        "Session s\nstate a(1.5)\nstate b(2)\nlet c = [a, b]\nSynth y:\n    Osc | freq=(c[0])\n";
    let warned: Vec<Code> = support::checked_all(passed_over)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .filter(|code| *code == Code::IntegerCell)
        .collect();
    if !warned.is_empty() {
        failures.push(format!("{passed_over}warned {warned:?}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_array_literal_holds_one_element_type_and_one_family_wherever_it_stands() {
    let rows: [Row; 18] = [
        (
            &[
                "let a = [1, \"s\"]",
                "Clip c:\n    for e in a:\n        Note",
            ],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &["let a = [Sine, 3]", "Clip c:\n    Note/(a[1])"],
            &[(Code::TypeMismatch, "[Sine, 3]")],
        ),
        (
            &[
                "let a = [1bars, 2ms]",
                "Clip c:\n    for e in a:\n        Note",
            ],
            &[(Code::MixedFamilies, "[1bars, 2ms]")],
        ),
        (
            &["Clip c:\n    for e in [1, \"s\"]:\n        Note"],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &[
                "let a = [1, \"s\"]",
                "Clip c:\n    for e in a:\n        Note",
            ],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &[
                "let g = [[1, 2], [3, \"s\"]]",
                "Clip c:\n    Note/(g[0][0])",
            ],
            &[(Code::TypeMismatch, "[3, \"s\"]")],
        ),
        (
            &[
                "let a = [1, 2]",
                "let b = [\"s\", \"t\"]",
                "let c = [a, b]",
                "Clip k:\n    Note/(c[0][0])",
            ],
            &[(Code::TypeMismatch, "[a, b]")],
        ),
        (
            &[
                "let a = [1, \"s\"]",
                "let b = [a, a]",
                "Clip c:\n    Note/(b[0][0])",
            ],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &["fn f(a): a[0]", "Clip c:\n    Note/(f([1, \"s\"]))"],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &["fn f(a = [1, \"s\"]): a[0]", "Clip c:\n    Note/(f())"],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &[
                "pattern P(a):\n    Note/(a[0])",
                "Clip c:\n    P | a=[1, \"s\"]",
            ],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &["Clip c | gain=([1, \"s\"][0])"],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &["state k([1, Wave::Saw])"],
            &[(Code::TypeMismatch, "[1, Wave::Saw]")],
        ),
        (
            &["Clip c:\n    Note/(if true then [1, \"s\"][0] else 2)"],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &["state f(true)", "let q = if f then [1, \"s\"] else [1, 2]"],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (
            &[
                "let q = match true:\n    true: [1, \"s\"]\n    false: [1, 2]",
                "Clip c:\n    for e in q:\n        Note",
            ],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
        (&["let a = [1, 2.5]"], &[]),
        (
            &["let a = [1, 3000000000]", "Clip c:\n    Note/(a[0])"],
            &[],
        ),
    ];
    let mut failures = Vec::new();
    for (items, wanted) in rows {
        let wanted: Errors = wanted
            .iter()
            .map(|(code, span)| (*code, (*span).to_string()))
            .collect();
        let all: Vec<&'static str> = std::iter::once("Session s")
            .chain(items.iter().copied())
            .collect();
        for order in support::permutations(&all) {
            let text = format!("{}\n", order.join("\n"));
            let found = fixture_errors(&text);
            if found != wanted {
                failures.push(format!("{text}expected {wanted:?}\nfound {found:?}"));
                break;
            }
        }
    }
    for (iterated, read, indexed) in [
        (
            "Session s\nClip c:\n    for e in [60, 2.5]:\n        Note/(e)\n",
            "e",
            "Session s\nClip c:\n    for k in 0..2:\n        Note/([60, 2.5][k])\n",
        ),
        (
            "Session s\nlet a = [60, 2.5]\nClip c:\n    for e in a:\n        Note/(e)\n",
            "e",
            "Session s\nlet a = [60, 2.5]\nClip c:\n    for k in 0..2:\n        Note/(a[k])\n",
        ),
    ] {
        let by_element = fixture_errors(iterated);
        let by_index = codes(indexed);
        if by_element != [(Code::TypeMismatch, read.to_string())]
            || by_index != [Code::TypeMismatch]
        {
            failures.push(format!(
                "{iterated}drew {by_element:?}\n{indexed}drew {by_index:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn an_empty_array_has_length_zero_and_a_compile_time_index_into_it_is_outside() {
    let rows: [Row; 14] = [
        (&["state k([]): i32[2]"], &[(Code::TypeMismatch, "[]")]),
        (
            &["state k([[], []]): i32[2][2]"],
            &[(Code::TypeMismatch, "[[], []]")],
        ),
        (&["state k([1, 2]): i32[2]"], &[]),
        (
            &["state k([1, 2]): i32[3]"],
            &[(Code::TypeMismatch, "[1, 2]")],
        ),
        (
            &["let v = [[]]", "Clip c:\n    Note/(v[0][0])"],
            &[(Code::IndexOutOfRange, "0")],
        ),
        (
            &["let v = [[]]", "Clip c:\n    Note/(v[0])"],
            &[(Code::TypeMismatch, "v[0]")],
        ),
        (
            &["Clip c:\n    Note/([[]][0][0])"],
            &[(Code::IndexOutOfRange, "0")],
        ),
        (
            &["let v = [[], []]", "Clip c:\n    Note/(v[1][0])"],
            &[(Code::IndexOutOfRange, "0")],
        ),
        (
            &["state i(0)", "let v = [[]]", "Clip c:\n    Note/(v[0][i])"],
            &[],
        ),
        (
            &["state k(1): i32[0]", "Clip c:\n    Note/(k[0])"],
            &[(Code::ZeroLength, "0")],
        ),
        (
            &["let k: i32[0] = 1", "Clip c:\n    Note/(k[0])"],
            &[(Code::ZeroLength, "0")],
        ),
        (
            &[
                "let a = [[60, 64], [67, 71]]",
                "Clip c:\n    Note/(a[1][0])\n    Note/(a[0][1])",
            ],
            &[],
        ),
        (
            &[
                "let a = [[60], [67, 71]]",
                "Clip c:\n    for e in a:\n        Note",
            ],
            &[(Code::RaggedArray, "[[60], [67, 71]]")],
        ),
        (
            &["Clip c:\n    Note/([[60], [67, 71]][0][0])"],
            &[(Code::RaggedArray, "[[60], [67, 71]]")],
        ),
    ];
    let mut failures = Vec::new();
    for (items, wanted) in rows {
        let wanted: Errors = wanted
            .iter()
            .map(|(code, span)| (*code, (*span).to_string()))
            .collect();
        let all: Vec<&'static str> = std::iter::once("Session s")
            .chain(items.iter().copied())
            .collect();
        for order in support::permutations(&all) {
            let text = format!("{}\n", order.join("\n"));
            let found = fixture_errors(&text);
            if found != wanted {
                failures.push(format!("{text}expected {wanted:?}\nfound {found:?}"));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

type Located = Vec<(Code, String, Vec<String>)>;

type LocatedRow = (
    &'static [&'static str],
    &'static [(Code, &'static str, &'static [&'static str])],
);

fn located_errors(text: &str) -> Located {
    let excerpt = |range: &std::ops::Range<u32>| {
        let at = usize::try_from(range.start).unwrap_or(0)..usize::try_from(range.end).unwrap_or(0);
        text.get(at).unwrap_or("").to_string()
    };
    let mut found: Located = support::checked(text)
        .iter()
        .map(|diagnostic| {
            let secondary = diagnostic
                .related
                .iter()
                .map(|related| &related.span)
                .map(|span| excerpt(&span.range))
                .collect();
            (diagnostic.code, excerpt(&diagnostic.span.range), secondary)
        })
        .collect();
    found.sort();
    found
}

fn with_session(items: &[&'static str]) -> Vec<&'static str> {
    std::iter::once("Session s")
        .chain(items.iter().copied())
        .collect()
}

fn first_order_failing<W: PartialEq + std::fmt::Debug>(
    items: &[&'static str],
    wanted: &W,
    found: impl Fn(&str) -> W,
) -> Option<String> {
    first_order_failing_in(support::Orders::Every, items, wanted, found)
}

fn first_order_failing_in<W: PartialEq + std::fmt::Debug>(
    orders: support::Orders,
    items: &[&'static str],
    wanted: &W,
    found: impl Fn(&str) -> W,
) -> Option<String> {
    orders.of(items).into_iter().find_map(|order| {
        let text = format!("{}\n", order.join("\n"));
        let found = found(&text);
        (found != *wanted).then(|| format!("{text}expected {wanted:?}\nfound {found:?}"))
    })
}

fn expect_in_every_order(rows: &[Row]) {
    expect_in_orders(support::Orders::Every, rows);
}

fn expect_in_orders(orders: support::Orders, rows: &[Row]) {
    let failures: Vec<String> = rows
        .iter()
        .filter_map(|(items, wanted)| {
            let mut wanted: Errors = wanted
                .iter()
                .map(|(code, span)| (*code, (*span).to_string()))
                .collect();
            wanted.sort();
            first_order_failing_in(orders, &with_session(items), &wanted, fixture_errors)
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn expect_located_in_every_order(rows: &[LocatedRow]) {
    expect_located_in_orders(support::Orders::Every, rows);
}

fn expect_located_in_orders(orders: support::Orders, rows: &[LocatedRow]) {
    expect_located_with_in(orders, rows, with_session, located_errors);
}

fn expect_located_with(
    rows: &[LocatedRow],
    items_of: fn(&[&'static str]) -> Vec<&'static str>,
    found: fn(&str) -> Located,
) {
    expect_located_with_in(support::Orders::Every, rows, items_of, found);
}

fn expect_located_with_in(
    orders: support::Orders,
    rows: &[LocatedRow],
    items_of: fn(&[&'static str]) -> Vec<&'static str>,
    found: fn(&str) -> Located,
) {
    let failures: Vec<String> = rows
        .iter()
        .filter_map(|(items, wanted)| {
            let mut wanted: Located = wanted
                .iter()
                .map(|(code, span, secondary)| {
                    (
                        *code,
                        (*span).to_string(),
                        secondary.iter().map(|span| (*span).to_string()).collect(),
                    )
                })
                .collect();
            wanted.sort();
            first_order_failing_in(orders, &items_of(items), &wanted, found)
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn a_literal_whose_elements_disagree_draws_one_diagnostic_wherever_its_value_goes_in(
    orders: support::Orders,
) {
    const H: &str = "fn h(): [1, \"s\"]";
    const R: &str = "fn r(n): if n < 1 then [1, \"s\"] else r(n - 1)";
    const R_OK: &str = "fn r(n): if n < 1 then [1, 2] else r(n - 1)";
    const LITERAL: &str = "[1, \"s\"]";
    let at_literal_of_h: &[(Code, &str, &[&str])] = &[(Code::TypeMismatch, LITERAL, &["h()"])];
    let at_literal_of_r: &[(Code, &str, &[&str])] = &[(Code::TypeMismatch, LITERAL, &["r(2)"])];
    const F: &str = "fn f(n, a): if n < 1 then a[0] else f(n - 1, a)";
    let rows: [LocatedRow; 45] = [
        (&[H, "Clip c:\n    Note/(h()[0])"], at_literal_of_h),
        (
            &[H, "state f(true)", "let q = if f then h() else [1, 2, 3]"],
            at_literal_of_h,
        ),
        (&[H, "state k(h())"], at_literal_of_h),
        (
            &[
                H,
                "let q = match true:\n    true: h()\n    false: [1, 2]",
                "Clip c:\n    Note/(q[0])",
            ],
            at_literal_of_h,
        ),
        (&[H, F, "Clip c:\n    Note/(f(2, h()))"], at_literal_of_h),
        (
            &[
                F,
                "Clip c:\n    for i in 0..f(2, [1, \"s\"]):\n        Note",
            ],
            &[(Code::TypeMismatch, LITERAL, &[])],
        ),
        (
            &[
                "fn f(n, a): if n < 1 then a else f(n - 1, a)",
                "Clip c:\n    for e in f(2, [1, \"s\"]):\n        Note",
            ],
            &[(Code::TypeMismatch, LITERAL, &[])],
        ),
        (
            &[
                "fn f(n, a): if n < 1 then a[0][0] else f(n - 1, a)",
                "Clip c:\n    Note/(f(2, [[1, 2], [3, \"s\"]]))",
            ],
            &[(Code::TypeMismatch, "[3, \"s\"]", &[])],
        ),
        (
            &[
                "fn r(n): if n < 1 then [1, \"s\"] else (if n < 5 then [3, 4] else r(n - 1))",
                "Clip c:\n    Note/(r(2)[0])",
            ],
            at_literal_of_r,
        ),
        (
            &[
                "fn r(n):\n    let a = [1, \"s\"]\n    if n < 1 then a else r(n - 1)",
                "Clip c:\n    Note/(r(2)[0])",
            ],
            at_literal_of_r,
        ),
        (
            &[H, "let a = h()", "Clip c:\n    Note/(a[0])"],
            at_literal_of_h,
        ),
        (
            &[H, "Clip c:\n    for e in h():\n        Note"],
            at_literal_of_h,
        ),
        (&[H, "Clip c:\n    Note/(h())"], at_literal_of_h),
        (
            &[H, "state i(0)", "Clip c:\n    Note/(h()[i])"],
            at_literal_of_h,
        ),
        (
            &["fn h(): [1bars, 2ms]", "Clip c:\n    Note/(h()[0])"],
            &[(Code::MixedFamilies, "[1bars, 2ms]", &["h()"])],
        ),
        (
            &["fn h(x): [x, 2]", "Clip c:\n    Note/(h(\"s\")[0])"],
            &[(Code::TypeMismatch, "[x, 2]", &["h(\"s\")"])],
        ),
        (
            &[H, "Clip c:\n    Note/(if true then 1 else h()[0])"],
            at_literal_of_h,
        ),
        (
            &[
                H,
                "state f(true)",
                "let q = if f then h() else [1, 2]",
                "Clip c:\n    Note/(q[0])",
            ],
            at_literal_of_h,
        ),
        (
            &["let a = [1, \"s\"]", "Clip c:\n    Note/(a)"],
            &[(Code::TypeMismatch, LITERAL, &[])],
        ),
        (&[R, "Clip c:\n    Note/(r(2)[0])"], at_literal_of_r),
        (
            &[R, "let a = r(2)", "Clip c:\n    Note/(r(2)[0])"],
            at_literal_of_r,
        ),
        (
            &[
                "fn r(n):\n    match n < 1:\n        true: [1, \"s\"]\n        false: r(n - 1)",
                "Clip c:\n    Note/(r(2)[0])",
            ],
            at_literal_of_r,
        ),
        (
            &[R, "Clip c:\n    for e in r(2):\n        Note"],
            at_literal_of_r,
        ),
        (
            &[
                "fn r(n, x): if n < 1 then [x, 1] else r(n - 1, x)",
                "Clip c:\n    Note/(r(2, \"s\")[0])",
            ],
            &[(Code::TypeMismatch, "[x, 1]", &["r(2, \"s\")"])],
        ),
        (
            &[
                "fn r(n): if n < 1 then 0 else [r(n - 1), \"s\"][0]",
                "Clip c:\n    Note/(r(2))",
            ],
            &[(Code::TypeMismatch, "[r(n - 1), \"s\"]", &["r(2)"])],
        ),
        (
            &[F, "Clip c:\n    Note/(f(2, [1, \"s\"]))"],
            &[(Code::TypeMismatch, LITERAL, &[])],
        ),
        (
            &[
                R_OK,
                "let b = [r(1), [3, \"s\"]]",
                "Clip c:\n    Note/(b[0][0])",
            ],
            &[(Code::TypeMismatch, "[3, \"s\"]", &[])],
        ),
        (
            &[
                R_OK,
                "let b = [[3, \"s\"], r(1)]",
                "Clip c:\n    Note/(b[0][0])",
            ],
            &[(Code::TypeMismatch, "[3, \"s\"]", &[])],
        ),
        (
            &[
                R_OK,
                "let x = r(1)",
                "let b = [x, [3, \"s\"]]",
                "Clip c:\n    Note/(b[0][0])",
            ],
            &[(Code::TypeMismatch, "[3, \"s\"]", &[])],
        ),
        (
            &[
                R_OK,
                "let b = [r(1), [3bars, 4ms]]",
                "Clip c:\n    Note/(b[0][0])",
            ],
            &[(Code::MixedFamilies, "[3bars, 4ms]", &[])],
        ),
        (
            &[H, "Clip c:\n    Note/(h()[0])\n    Note/(h()[1])"],
            &[
                (Code::TypeMismatch, LITERAL, &["h()"]),
                (Code::TypeMismatch, LITERAL, &["h()"]),
            ],
        ),
        (&["fn h(): [1, 2]", "Clip c:\n    Note/(h()[0])"], &[]),
        (
            &[
                "fn h(): [1, 2]",
                "let a = h()",
                "Clip c:\n    for e in a:\n        Note/(e)",
            ],
            &[],
        ),
        (&["fn h(x): x", "Clip c:\n    Note/(h([60, 64])[1])"], &[]),
        (
            &["let a = [1, 2]", "Clip c:\n    Note/(a)"],
            &[(Code::TypeMismatch, "a", &[])],
        ),
        (
            &[
                "fn f(n): if n < 1 then [60, 64] else f(n - 1)",
                "Clip c:\n    for e in f(2):\n        Note/(e)\n    Note/(f(2)[1])",
            ],
            &[],
        ),
        (&[F, "Clip c:\n    Note/(f(2, [1, 2]))"], &[]),
        (
            &[
                "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)",
                "Clip c:\n    Note/(fib(6))",
            ],
            &[],
        ),
        (
            &[
                R_OK,
                "let b = [r(1), [3, 4]]",
                "Clip c:\n    Note/(b[0][0])\n    Note/(b[1][1])",
            ],
            &[],
        ),
        (
            &[
                R_OK,
                "let b = [r(1), [\"s\", \"t\"]]",
                "Clip c:\n    Note/(b[0][0])",
            ],
            &[(Code::TypeMismatch, "[r(1), [\"s\", \"t\"]]", &[])],
        ),
        (
            &[
                "fn f(n): if n < 1 then [1, 2.5][0] else f(n - 1)",
                "Clip c:\n    Note/(f(2))",
            ],
            &[(Code::TypeMismatch, "f(2)", &[])],
        ),
        (
            &["let a = [60, 64]", "let b = a", "Clip c:\n    Note/(b[1])"],
            &[],
        ),
        (
            &[
                "let a = [60, 64]",
                "let b = [a, a]",
                "Clip c:\n    Note/(b[1][0])",
            ],
            &[],
        ),
        (
            &[
                "fn f(x, i): x[i]",
                "let a = [60, 64]",
                "Clip c:\n    Note/(f(a, 1))",
            ],
            &[],
        ),
        (
            &[
                "fn f(x, i): x[i]",
                "let a = [60, \"s\"]",
                "Clip c:\n    Note/(f(a, 1))",
            ],
            &[(Code::TypeMismatch, "[60, \"s\"]", &[])],
        ),
    ];
    expect_located_in_orders(orders, &rows);
}

#[test]
fn a_literal_whose_elements_disagree_draws_one_diagnostic_wherever_its_value_goes() {
    a_literal_whose_elements_disagree_draws_one_diagnostic_wherever_its_value_goes_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_literal_whose_elements_disagree_draws_one_diagnostic_wherever_its_value_goes() {
    a_literal_whose_elements_disagree_draws_one_diagnostic_wherever_its_value_goes_in(
        support::Orders::Every,
    );
}

#[test]
fn a_plain_element_of_a_unit_literal_in_an_arm_is_read_in_the_unit_its_family_value_reaches() {
    const Q: &str = "let q = if true then [1bars, 2] else [3bars, 4]";
    const AT_LENGTH: &str = "Clip k:\n    Note | length=(q[1])";
    let rows: [Row; 10] = [
        (&[Q, AT_LENGTH], &[]),
        (
            &[
                "state c(true)",
                "let q = if c then [1bars, 2] else [3bars, 4]",
                AT_LENGTH,
            ],
            &[],
        ),
        (
            &[
                "let q = match true:\n    true: [1bars, 2]\n    false: [3bars, 4]",
                AT_LENGTH,
            ],
            &[],
        ),
        (
            &[Q, "Clip k:\n    for e in q:\n        Note | length=(e)"],
            &[],
        ),
        (&[Q, "let r = q", "Clip k:\n    Note | length=(r[1])"], &[]),
        (&[Q, "state i(0)", "Clip k:\n    Note | length=(q[i])"], &[]),
        (
            &[
                "let q = if true then 2 else 3bars",
                "Clip k:\n    Note | length=q",
            ],
            &[],
        ),
        (
            &[Q, "Clip k:\n    Note/(q[1])"],
            &[
                (Code::PlainInFamily, "2"),
                (Code::PlainInFamily, "4"),
                (Code::UnitMismatch, "q[1]"),
            ],
        ),
        (
            &[
                "let q = if true then [1bars, 2] else [3ms, 4]",
                "Clip k:\n    for e in q:\n        Note",
            ],
            &[(Code::MixedFamilies, "if true then [1bars, 2] else [3ms, 4]")],
        ),
        (
            &["let q = if true then [1bars, 2] else [3ms, 4]", AT_LENGTH],
            &[(Code::MixedFamilies, "if true then [1bars, 2] else [3ms, 4]")],
        ),
    ];
    expect_in_every_order(&rows);
}

#[test]
fn a_for_elements_uses_are_the_uses_of_one_flexible_value() {
    let rows: [LocatedRow; 9] = [
        (
            &[
                "let a = [2 ** -1, 1]",
                "Clip c:\n    for e in a:\n        Note/(e) | brightness=(e)",
            ],
            &[(Code::InconsistentUses, "e", &["e"])],
        ),
        (
            &[
                "let a = [2 ** -1, 1]",
                "Clip c:\n    for k in 0..2:\n        Note/(a[k]) | brightness=(a[k])",
            ],
            &[(Code::InconsistentUses, "a[k]", &["a[k]"])],
        ),
        (
            &["Clip c:\n    for e in [2 ** -1, 1]:\n        Note/(e) | brightness=(e)"],
            &[(Code::InconsistentUses, "e", &["e"])],
        ),
        (
            &[
                "pattern P(a):\n    for e in a:\n        Note/(e) | brightness=(e)",
                "Clip c:\n    P | a=[2 ** -1, 1]",
            ],
            &[(Code::InconsistentUses, "e", &["e", "P | a=[2 ** -1, 1]"])],
        ),
        (
            &["Clip c:\n    for e in [2 ** -1, 1]:\n        Note/(e + 0) | brightness=(e + 0)"],
            &[(Code::InconsistentUses, "e", &["e"])],
        ),
        (
            &["Clip c:\n    for e in [2 ** 1, 1]:\n        Note/(e) | brightness=(e)"],
            &[],
        ),
        (
            &["Clip c:\n    for e in [1, 2]:\n        Note/(e) | brightness=(e)"],
            &[],
        ),
        (
            &["Clip c:\n    for k in 0..2:\n        Note/(k) | brightness=(k)"],
            &[],
        ),
        (
            &["Clip c:\n    for e in [2 ** -1, 1]:\n        Note | brightness=(e) length=(e)"],
            &[],
        ),
    ];
    expect_located_in_every_order(&rows);
}

#[test]
fn a_diagnostic_about_one_iterations_value_points_where_the_body_reads_it() {
    const FLOAT_AT_PITCH: Code = Code::TypeMismatch;
    let rows: [LocatedRow; 34] = [
        (
            &["Clip c:\n    for e in [60, 2.5]:\n        Note/(e)\n        Note/(e)"],
            &[(FLOAT_AT_PITCH, "e", &[]), (FLOAT_AT_PITCH, "e", &[])],
        ),
        (
            &[
                "Clip c:\n    for k in 0..2:\n        Note/([60, 2.5][k])\n        Note/([60, 2.5][k])",
            ],
            &[
                (FLOAT_AT_PITCH, "[60, 2.5][k]", &[]),
                (FLOAT_AT_PITCH, "[60, 2.5][k]", &[]),
            ],
        ),
        (
            &["Clip c:\n    for e in [1, 3000000000]:\n        Note/(e)"],
            &[(Code::DoesNotFit, "e", &[])],
        ),
        (
            &["Clip c:\n    for k in 0..2:\n        Note/([1, 3000000000][k])"],
            &[(Code::DoesNotFit, "[1, 3000000000][k]", &[])],
        ),
        (
            &["Clip c:\n    for p in [2, 9]:\n        Note/([60, 64, 67, 71][p])"],
            &[(Code::IndexOutOfRange, "p", &[])],
        ),
        (
            &["Clip c:\n    for k in 0..2:\n        Note/([60, 64, 67, 71][[2, 9][k]])"],
            &[(Code::IndexOutOfRange, "[2, 9][k]", &[])],
        ),
        (
            &["Clip c:\n    for e in [60, 2.5]:\n        Note/(e + 1)"],
            &[(FLOAT_AT_PITCH, "e + 1", &[])],
        ),
        (
            &["Clip c:\n    for k in 0..2:\n        Note/([60, 2.5][k] + 1)"],
            &[(FLOAT_AT_PITCH, "[60, 2.5][k] + 1", &[])],
        ),
        (
            &["Clip c:\n    for e in [2147483647, 2147483648]:\n        Note/(e)"],
            &[(Code::DoesNotFit, "e", &[])],
        ),
        (
            &["Clip c:\n    for k in 0..2:\n        Note/([2147483647, 2147483648][k])"],
            &[(Code::DoesNotFit, "[2147483647, 2147483648][k]", &[])],
        ),
        (
            &["Clip c:\n    for e in [1ms, 2ms]:\n        Note | length=e"],
            &[(Code::UnitMismatch, "e", &[])],
        ),
        (
            &["Clip c:\n    for k in 0..2:\n        Note | length=([1ms, 2ms][k])"],
            &[(Code::UnitMismatch, "[1ms, 2ms][k]", &[])],
        ),
        (
            &[
                "let a = [60, 2.5]",
                "Clip c:\n    for e in a:\n        Note/(e)",
            ],
            &[(FLOAT_AT_PITCH, "e", &[])],
        ),
        (
            &[
                "let a = [60, 2.5]",
                "Clip c:\n    for k in 0..2:\n        Note/(a[k])",
            ],
            &[(FLOAT_AT_PITCH, "a[k]", &[])],
        ),
        (
            &["Clip c:\n    for e in [60, 64]:\n        Note/(e) | velocity=e"],
            &[],
        ),
        (
            &[
                "Clip c:\n    for e in [60, 64]:\n        Note/(e)",
                "Synth y:\n    for e in [60.5, 64]:\n        Osc | freq=e",
            ],
            &[],
        ),
        (
            &["Kit k:\n    for f in [\"a\", \"b\"]:\n        Sampler | file=\"{f}.wav\""],
            &[],
        ),
        (
            &["Kit k:\n    for f in [\"a.wav\", \"b.wav\"]:\n        Sampler | file=f"],
            &[],
        ),
        (
            &[
                "Synth y:\n    for w in [Wave::Sine, Wave::Saw]:\n        Osc | wave=w\n        Osc | rate=(if w is Saw then 1 else 2)",
            ],
            &[],
        ),
        (
            &[
                "Clip a:\n    Note",
                "Clip b:\n    Note",
                "Clip c | gain=0.5:\n    for t in [a, b]:\n        Note | velocity=t.gain",
            ],
            &[],
        ),
        (
            &[
                "pattern P(target):\n    Plays: target",
                "Clip a:\n    for t in [b, c]:\n        P | target=t",
                "Clip b:\n    Note",
                "Clip c:\n    Note",
            ],
            &[],
        ),
        (
            &[
                "Clip a:\n    for t in [b, c]:\n        Plays: t\n    Plays: [b, c][1]",
                "Clip b:\n    Note",
                "Clip c:\n    Note",
            ],
            &[],
        ),
        (
            &["Clip a:\n    for p in [60, 64]:\n        for q in [p, 67]:\n            Note/(q)"],
            &[],
        ),
        (
            &["Clip a:\n    for p in [60, 64]:\n        for i in 0..p:\n            Note"],
            &[],
        ),
        (
            &[
                "fn r(n): if n < 1 then 0 else r(n - 1) + 1",
                "Clip a:\n    for p in [2, 3]:\n        Note/(r(p))",
            ],
            &[],
        ),
        (
            &["Clip a:\n    for p in [2, 3]:\n        Note/([60, 64, 67, 71][p])"],
            &[],
        ),
        (
            &["Kit k:\n    for f in [\"a\", \"b\"]:\n        Sampler | file=f + \".wav\""],
            &[],
        ),
        (
            &[
                "pattern P(default n):\n    Note/(n)",
                "Clip a:\n    for p in [60, 64]:\n        P/(p)",
            ],
            &[],
        ),
        (
            &["Clip a:\n    for w in [Wave::Sine, Wave::Saw]:\n        Note when w is Saw"],
            &[],
        ),
        (
            &[
                "let m = 3",
                "Clip a:\n    for p in [m, 64]:\n        Note/(p)",
            ],
            &[],
        ),
        (
            &["Clip a:\n    for p in [1, 2]:\n        Note/(p) | length=(p * 1bars)"],
            &[],
        ),
        (
            &["Clip a:\n    for p in [true, false]:\n        Note when p\n        Note | accent=p"],
            &[],
        ),
        (
            &[
                "let sections = [b, c]",
                "Clip a:\n    for t in sections:\n        Plays: t",
                "Clip b:\n    Note",
                "Clip c:\n    Note",
            ],
            &[],
        ),
        (
            &["Synth y:\n    for e in [16777217, 1.5]:\n        Osc | rate=e"],
            &[(Code::DoesNotFit, "16777217", &[])],
        ),
    ];
    expect_located_in_every_order(&rows);
}

#[test]
fn a_default_no_call_takes_and_a_for_body_that_runs_no_iteration_are_unchecked() {
    let rows: [Row; 8] = [
        (
            &["fn f(a = zz): 1", "Clip c:\n    Note/(f(2))"],
            &[(Code::UnknownName, "zz")],
        ),
        (
            &["Clip c:\n    for i in 0..0:\n        Note/(zz)"],
            &[(Code::UnknownName, "zz")],
        ),
        (
            &["pattern P(a = zz):\n    Note", "Clip c:\n    P | a=1"],
            &[(Code::UnknownName, "zz")],
        ),
        (
            &["Clip c:\n    for i in 0..0:\n        Plays: zz"],
            &[(Code::UnknownName, "zz")],
        ),
        (
            &["fn f(a = [1, \"s\"]): 1", "Clip c:\n    Note/(f(2))"],
            &[],
        ),
        (&["fn f(a = \"s\" + 1): 1", "Clip c:\n    Note/(f(2))"], &[]),
        (
            &["Clip c:\n    for i in 0..0:\n        Note/(i + \"s\")"],
            &[],
        ),
        (
            &["fn f(a = [1, \"s\"]): 1", "Clip c:\n    Note/(f())"],
            &[(Code::TypeMismatch, "[1, \"s\"]")],
        ),
    ];
    expect_in_every_order(&rows);
}

fn bar_errors(text: &str) -> Located {
    let compiler = Compiler::new(&symphonyscript_fixtures::bar_vocabulary())
        .expect("the bar vocabulary compiles");
    let excerpt = |range: &std::ops::Range<u32>| {
        let at = usize::try_from(range.start).unwrap_or(0)..usize::try_from(range.end).unwrap_or(0);
        text.get(at).unwrap_or("").to_string()
    };
    let mut found: Located = support::errors(compiler.check(&[support::source(text)]))
        .iter()
        .map(|diagnostic| {
            let secondary = diagnostic
                .related
                .iter()
                .map(|related| &related.span)
                .map(|span| excerpt(&span.range))
                .collect();
            (diagnostic.code, excerpt(&diagnostic.span.range), secondary)
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_literals_family_value_reaches_every_attribute_a_read_of_it_reaches() {
    const A: &str = "let a = [1bars, 2]";
    const Q: &str = "let q = if true then [1bars, 2] else [3bars, 4]";
    let rows: [Row; 9] = [
        (&[A, "Clip k:\n    Note | length=(a[0])"], &[]),
        (&[Q, "Clip k:\n    Note | length=(q[0])"], &[]),
        (
            &[A, "Clip k:\n    for e in a:\n        Note | length=(e)"],
            &[],
        ),
        (&[A, "state i(0)", "Clip k:\n    Note | length=(a[i])"], &[]),
        (
            &["fn f(x): x[0]", A, "Clip k:\n    Note | length=(f(a))"],
            &[],
        ),
        (&["Clip k:\n    Note | length=([1bars, 2][0])"], &[]),
        (
            &[
                "let g = [[1bars, 2], [3bars, 4]]",
                "Clip k:\n    Note | length=(g[1][0])",
            ],
            &[],
        ),
        (
            &[A, "Clip k:\n    for e in a:\n        Note"],
            &[(Code::PlainInFamily, "2")],
        ),
        (
            &[Q, "Clip k:\n    for e in q:\n        Note"],
            &[(Code::PlainInFamily, "2"), (Code::PlainInFamily, "4")],
        ),
    ];
    expect_in_every_order(&rows);
    let items = [A, "Bar b1 | bar_len=(a[0])", "Bar b2 | beat_len=(a[0])"];
    let wanted: Located = vec![(
        Code::ConflictingUnits,
        "2".to_string(),
        vec!["a[0]".to_string(), "a[0]".to_string()],
    )];
    let failure = first_order_failing(&items, &wanted, bar_errors);
    assert!(failure.is_none(), "{}", failure.unwrap_or_default());
}

#[test]
fn a_construct_missing_its_block_draws_the_same_diagnostics_in_every_order() {
    let rows: [(&[&'static str], &[Code]); 5] = [
        (
            &[
                "Session main /128:",
                "pattern Arp(default root, steps = 4, gap = 4):",
                "Clip intro | gain=0.9:",
            ],
            &[
                Code::PlacementSyntax,
                Code::PlacementSyntax,
                Code::PlacementSyntax,
                Code::UnreachableInstance,
            ],
        ),
        (
            &[
                "Session main:\n    Includes: t",
                "Track t",
                "fn f():",
                "Clip c:\n    Note",
                "let a = 1",
            ],
            &[Code::FnSyntax, Code::UnreachableInstance],
        ),
        (
            &[
                "Session main:\n    Includes: t",
                "Track t",
                "let m = match 1:",
                "Clip c:\n    Note",
            ],
            &[Code::MatchSyntax, Code::UnreachableInstance],
        ),
        (
            &[
                "Session main:\n    Includes: t",
                "Track t",
                "Clip c:\n    when true:",
                "Clip d:\n    Note",
            ],
            &[
                Code::PlacementSyntax,
                Code::UnreachableInstance,
                Code::UnreachableInstance,
            ],
        ),
        (
            &["Track t:", "fn g(x): x", "Session s:\n    Includes: t"],
            &[Code::PlacementSyntax],
        ),
    ];
    let failures = codes_in_every_order(&rows, support::checked_all);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

type CodeRow = (&'static [&'static str], &'static [Code]);

fn codes_in_every_order(rows: &[CodeRow], check: fn(&str) -> Vec<Diagnostic>) -> Vec<String> {
    let mut failures = Vec::new();
    for (items, wanted) in rows {
        let mut wanted = wanted.to_vec();
        wanted.sort();
        for order in support::permutations(items) {
            let text = format!("{}\n", order.join("\n"));
            let mut found: Vec<Code> = check(&text)
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect();
            found.sort();
            if found != wanted {
                failures.push(format!("{text}expected {wanted:?}\nfound {found:?}"));
                break;
            }
        }
    }
    failures
}

#[test]
fn a_pattern_whose_block_a_fault_leaves_missing_is_a_placeholder_for_its_uses() {
    let rows: [CodeRow; 10] = [
        (
            &[
                "Session s",
                "pattern Loop(default root):",
                "Loop low /36",
                "Loop high /48",
            ],
            &[Code::PlacementSyntax],
        ),
        (
            &[
                "Session s",
                "pattern Loop(default root, again):",
                "Loop low /36 | again=low",
                "Clip c:\n    Plays: low",
            ],
            &[Code::PlacementSyntax],
        ),
        (
            &["Session s", "pattern P(x):", "P p1 | x=1"],
            &[Code::PlacementSyntax],
        ),
        (
            &["Session s", "pattern P(x):\n    = 3", "P p1 | x=1"],
            &[Code::PlacementSyntax],
        ),
        (
            &[
                "Session s",
                "pattern P(x):",
                "pattern Q(y):\n    P | x=y",
                "Q q1 | y=1",
            ],
            &[Code::PlacementSyntax],
        ),
        (
            &[
                "Session s",
                "pattern P(x):",
                "Clip c:\n    P as p1 | x=1\n    Note | velocity=p1.n.velocity",
            ],
            &[Code::PlacementSyntax],
        ),
        (
            &[
                "Session s",
                "pattern P(x):",
                "Clip d",
                "Clip c:\n    P as p1 | x=1\n    Plays: p1",
            ],
            &[Code::PlacementSyntax],
        ),
        (
            &["Session s", "pattern P(x):", "P p1 | x=1 y=2"],
            &[Code::PlacementSyntax, Code::UnknownParameter],
        ),
        (
            &["Session s", "pattern P(x):", "Clip c:\n    P | x=1"],
            &[Code::PlacementSyntax],
        ),
        (
            &[
                "Session s",
                "pattern P(x):\n    Note/x\n    Note/x",
                "P p1 | x=1",
                "P p2 | x=2",
            ],
            &[Code::TopLevelPattern, Code::TopLevelPattern],
        ),
    ];
    let mut failures = codes_in_every_order(&rows, support::checked);
    let unknown_children: [CodeRow; 1] = [(
        &["Session s:\n    P | x=1", "pattern P(x):", "Track t"],
        &[Code::PlacementSyntax],
    )];
    failures.extend(codes_in_every_order(
        &unknown_children,
        support::checked_all,
    ));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_use_of_a_name_a_skipped_region_may_declare_draws_nothing_more() {
    let rows: [Row; 19] = [
        (
            &["1 Clip d", "Clip c:\n    Plays: d"],
            &[(Code::TopLevelSyntax, "1")],
        ),
        (
            &["1 Clip d", "Clip c:\n    Plays: e"],
            &[(Code::TopLevelSyntax, "1"), (Code::UnknownName, "e")],
        ),
        (
            &[
                "Clip lead: Note __ as low",
                "Clip c:\n    Note | velocity=lead.low.velocity",
            ],
            &[(Code::InvalidIdentifier, "__")],
        ),
        (
            &[
                "Clip lead: Note Note | velocity=1 Note as low",
                "Clip c:\n    Note | velocity=lead.low.velocity",
            ],
            &[(Code::InlinePlacement, "|")],
        ),
        (
            &["pattern P(+ x):\n    Note", "Clip c:\n    P | x=1"],
            &[(Code::PatternSyntax, "+")],
        ),
        (
            &["fn f(x, + y):\n    x", "Clip c:\n    Note/(f(x=1))"],
            &[(Code::FnSyntax, "+")],
        ),
        (
            &[
                "Synth y:\n    Filter as f\n        Osc as o",
                "Osc sub:\n    Feeds: y.f.o",
            ],
            &[(Code::UnexpectedIndent, "Osc")],
        ),
        (
            &[
                "Synth lead:\n    __ as low\n    Filter as f",
                "Osc sub:\n    Feeds: lead.low",
            ],
            &[(Code::InvalidIdentifier, "__")],
        ),
        (
            &[
                "pattern P(default root + true, steps = 4):\n    Note/root",
                "Clip c:\n    P/1 | steps=2",
            ],
            &[(Code::PatternSyntax, "+")],
        ),
        (
            &[
                "pattern P(default root + true, steps = 4):\n    for i in 0..steps:\n        Note/root",
                "Clip c:\n    P/1",
            ],
            &[(Code::PatternSyntax, "+")],
        ),
        (
            &[
                "pattern P(x + default root):\n    Note/x",
                "Clip c:\n    P/1 | x=2",
            ],
            &[(Code::PatternSyntax, "+")],
        ),
        (
            &[
                "fn f(x, + y):\n    x + y",
                "Clip c:\n    Note/(f(1, 2, 3))\n    Note/(f(x=1, y=2))",
            ],
            &[(Code::FnSyntax, "+")],
        ),
        (
            &[
                "Synth lead\n    Filter as low",
                "Osc sub:\n    Feeds: lead.low",
            ],
            &[(Code::UnexpectedIndent, "Filter")],
        ),
        (
            &[
                "Synth lead:\n    = Filter as low",
                "Osc sub:\n    Feeds: lead.low",
            ],
            &[(Code::PlacementSyntax, "=")],
        ),
        (
            &["Synth lead:\n    = Filter as low\n    Osc:\n        Feeds: low"],
            &[(Code::PlacementSyntax, "=")],
        ),
        (
            &[
                "Synth lead:\n    Filter as low",
                "Osc sub:\n    Feeds: lead.low",
            ],
            &[],
        ),
        (
            &[
                "pattern P(default root):\n    Note/root",
                "Clip c:\n    P/1 | zork=2",
            ],
            &[(Code::UnknownParameter, "zork")],
        ),
        (
            &[
                "Synth lead\n    Filter as low",
                "Osc sub:\n    Feeds: lead.zork",
            ],
            &[
                (Code::UnexpectedIndent, "Filter"),
                (Code::UnknownAttribute, "zork"),
            ],
        ),
        (
            &["Clip lead:\n    = Note as low\n    Note | velocity=zz"],
            &[(Code::PlacementSyntax, "="), (Code::UnknownName, "zz")],
        ),
    ];
    expect_in_every_order(&rows);
}

#[test]
fn every_reference_to_a_top_level_name_declared_twice_is_a_placeholder() {
    let rows: [Row; 14] = [
        (&["state i(0)", "state i(i)"], &[(Code::DuplicateName, "i")]),
        (
            &["fn f(): 1", "fn f(x): x", "Clip c:\n    Note/(f(1))"],
            &[(Code::DuplicateName, "f")],
        ),
        (
            &[
                "state w(Wave::Saw)",
                "state w(Wave::Sine)",
                "Synth y:\n    Osc | wave=w",
            ],
            &[(Code::DuplicateName, "w")],
        ),
        (
            &["state a(2)", "let c = a", "state a(c)"],
            &[(Code::DuplicateName, "a")],
        ),
        (
            &[
                "pattern Riff(default root, velocity = 0.8):\n    Clip:\n        Plays: Riff",
                "pattern Riff.fill(default root):\n    Clip:",
                "Riff high /48 | velocity=bright",
            ],
            &[
                (Code::DuplicateName, "Riff"),
                (Code::PatternSyntax, "."),
                (Code::PlacementSyntax, ""),
                (Code::UnknownName, "bright"),
            ],
        ),
        (
            &[
                "state swing(0.1)",
                "let groove = swing",
                "Clip groove:\n    Plays: groove",
            ],
            &[(Code::DuplicateName, "groove")],
        ),
        (
            &[
                "fn lead(root, octave): root",
                "Track lead:\n    Plays: lead",
            ],
            &[(Code::DuplicateName, "lead")],
        ),
        (
            &[
                "pattern ascent():\n    Note",
                "Clip ascent:\n    Plays: ascent",
            ],
            &[(Code::DuplicateName, "ascent")],
        ),
        (
            &[
                "state a(1)",
                "Clip c:\n    Note | velocity=a",
                "state a(0.8)",
            ],
            &[(Code::DuplicateName, "a")],
        ),
        (
            &[
                "pattern P():\n    Riff",
                "pattern Riff():\n    Note",
                "fn Riff(): 1",
                "P p1",
            ],
            &[(Code::DuplicateName, "Riff")],
        ),
        (
            &[
                "pattern Q(x):\n    Note/x",
                "fn Q(): 1",
                "Clip c:\n    Q | x=1",
            ],
            &[(Code::DuplicateName, "Q")],
        ),
        (&["state i(i)"], &[(Code::DeclarationCycle, "i")]),
        (
            &["state i(0)", "state j(i)"],
            &[(Code::ReactiveInitializer, "i")],
        ),
        (
            &["state i(0)", "state i(1)", "state j(j)"],
            &[(Code::DeclarationCycle, "j"), (Code::DuplicateName, "i")],
        ),
    ];
    expect_in_every_order(&rows);
}

#[test]
fn a_recursive_calls_kind_settles_the_same_way_through_mutual_recursion() {
    const K: &str = "Clip k:\n    Note/(f(2))";
    const G: &str = "fn g(n): f(n)";
    let rows: [Row; 8] = [
        (
            &["fn f(n): if n < 1 then 0 else [g(n - 1), \"s\"][0]", G, K],
            &[(Code::TypeMismatch, "[g(n - 1), \"s\"]")],
        ),
        (
            &["fn f(n): if n < 1 then 0 else [f(n - 1), \"s\"][0]", K],
            &[(Code::TypeMismatch, "[f(n - 1), \"s\"]")],
        ),
        (
            &[
                "fn f(n): if n < 1 then 0 else [g(n - 1), \"s\"][0]",
                "fn g(n): h(n)",
                "fn h(n): f(n)",
                K,
            ],
            &[(Code::TypeMismatch, "[g(n - 1), \"s\"]")],
        ),
        (
            &["fn f(n): if n < 1 then 0 else [g(n - 1), 2][0]", G, K],
            &[],
        ),
        (
            &[
                "fn f(n): if n < 1 then 1 else [g(n - 1), 2][0] + 1",
                G,
                "Osc o | freq=(f(3))",
            ],
            &[],
        ),
        (
            &["fn f(n): if n < 1 then 0 else [g(n - 1), 2.5][0]", G, K],
            &[(Code::TypeMismatch, "f(2)")],
        ),
        (
            &["fn f(n): if n < 1 then 0 else [g(n - 1), \"s\"][1]", G, K],
            &[(
                Code::TypeMismatch,
                "if n < 1 then 0 else [g(n - 1), \"s\"][1]",
            )],
        ),
        (
            &["fn f(n): if n < 1 then 0 else [f(n - 1), \"s\"][1]", K],
            &[(
                Code::TypeMismatch,
                "if n < 1 then 0 else [f(n - 1), \"s\"][1]",
            )],
        ),
    ];
    expect_in_every_order(&rows);
}

const FAULTY_LINES: [&str; 51] = [
    "pattern P(:",
    "fn f(x, ): 1",
    "let = 3",
    "input : i32 = 1",
    "Clip d | =1",
    "Plays: c",
    "for i in :",
    "when x:",
    "let a: = 1",
    "let a: i32[ = 1",
    "Session s | tempo=1 velocity",
    "Clip d | gain=(1",
    "Clip d | gain=[1, 2",
    "fn f(x) 1",
    "fn (x): 1",
    "pattern (x):",
    "input x",
    "let a",
    "Clip",
    "Clip d |",
    "Clip d when",
    "Clip d when:",
    "Clip d/",
    "Clip d/(1",
    "Session s:",
    "Clip d: Note",
    "Clip d | gain=\"{",
    "let a = match",
    "let a = if 1 then",
    "let a = [",
    "fn f(x = ): 1",
    "Clip d !",
    "Clip d '",
    "state x(Sine): Wave",
    "let a = Wave::",
    "let a = x.",
    "Clip d as",
    "Session s | tempo=1 |",
    "fn f(a, a): a",
    "let a = (1, 2)",
    "Clip d: Note Note |",
    "Kit k: [Sampler",
    "Synth y: Osc | wave=Saw:",
    "Clip d | gain=1 when",
    "let a = 1 +",
    "let a = 1 + * 2",
    "let a = -",
    "let a = f(",
    "let a = f(1,",
    "let a = a[",
    "let a = 1..2",
];

#[test]
fn a_line_after_a_missing_block_reports_its_own_faults() {
    let rows: [Row; 12] = [
        (
            &["fn f(x):\n    let y = x", "Track t:\n    Note"],
            &[
                (Code::ExpressionSyntax, ""),
                (Code::ChildNotAccepted, "Note"),
            ],
        ),
        (
            &["Clip c:\nlet = 3"],
            &[(Code::PlacementSyntax, "let"), (Code::LetSyntax, "=")],
        ),
        (
            &["pattern P(x):\nlet = 3"],
            &[(Code::PlacementSyntax, "let"), (Code::LetSyntax, "=")],
        ),
        (
            &["fn f(x):\nlet = 3"],
            &[(Code::FnSyntax, "let"), (Code::LetSyntax, "=")],
        ),
        (
            &["let m = match 1:\nlet = 3"],
            &[(Code::MatchSyntax, "let"), (Code::LetSyntax, "=")],
        ),
        (
            &["Clip k:\n    when true:\n    Note |"],
            &[
                (Code::PlacementSyntax, "Note"),
                (Code::AttributeSyntax, "\n"),
            ],
        ),
        (
            &["Clip c:\nClip d | gain="],
            &[
                (Code::PlacementSyntax, "Clip"),
                (Code::ExpressionSyntax, "\n"),
            ],
        ),
        (
            &["Clip c:\nfn f(x, ): 1"],
            &[(Code::PlacementSyntax, "fn"), (Code::FnSyntax, ")")],
        ),
        (
            &["Clip c:\n= 3"],
            &[(Code::PlacementSyntax, "="), (Code::TopLevelSyntax, "=")],
        ),
        (
            &["Clip c:\n__ x"],
            &[
                (Code::PlacementSyntax, "__"),
                (Code::InvalidIdentifier, "__"),
            ],
        ),
        (
            &["Clip c:\nClip d\n    Note"],
            &[
                (Code::PlacementSyntax, "Clip"),
                (Code::UnexpectedIndent, "Note"),
            ],
        ),
        (
            &[
                "pattern Riff(default root):\n    Clip:\n\nRiff low 36",
                "Riff high /48",
            ],
            &[
                (Code::PlacementSyntax, ""),
                (Code::PlacementSyntax, "36"),
                (Code::MissingArgument, "Riff low 36"),
            ],
        ),
    ];
    expect_in_every_order(&rows);
    let counted: [CodeRow; 3] = [
        (
            &["Session s", "Clip c:\nSession s s"],
            &[
                Code::PlacementSyntax,
                Code::PlacementSyntax,
                Code::CountExceeded,
                Code::DuplicateName,
            ],
        ),
        (
            &["Clip c:\nClip d:", "Session s"],
            &[Code::PlacementSyntax, Code::PlacementSyntax],
        ),
        (
            &["Clip c:\nClip d:\nClip e:", "Session s"],
            &[
                Code::PlacementSyntax,
                Code::PlacementSyntax,
                Code::PlacementSyntax,
            ],
        ),
    ];
    let mut failures = codes_in_every_order(&counted, support::checked);
    for line in FAULTY_LINES {
        let first = line
            .split(|c: char| !c.is_alphanumeric())
            .next()
            .unwrap_or("");
        let after_missing = format!("Clip c:\n{line}");
        let after_valid = format!("Clip c:\n    Note\n{line}");
        let failure = support::permutations(&[0, 1, 2])
            .into_iter()
            .find_map(|order| {
                let text = |faulty: &str| {
                    let items = [faulty, "Session s", "Clip e:\n    Note"];
                    let ordered: Vec<&str> = order.iter().map(|index| items[*index]).collect();
                    format!("{}\n", ordered.join("\n"))
                };
                let (missing, valid) = (text(&after_missing), text(&after_valid));
                let mut wanted = fixture_errors(&valid);
                wanted.push((Code::PlacementSyntax, first.to_string()));
                wanted.sort();
                let found = fixture_errors(&missing);
                (found != wanted).then(|| format!("{missing}expected {wanted:?}\nfound {found:?}"))
            });
        failures.extend(failure);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn a_cell_read_through_a_literals_group_keeps_its_own_kind_in(orders: support::Orders) {
    const X: &str = "state x(1)";
    const C: &str = "state c(true)";
    const Q: &str = "let q = if c then [x, 2.5] else [3, 4]";
    let rows: [Row; 13] = [
        (&[X, C, Q], &[]),
        (&[X, "let a = [x, 2.5]", "Clip k:\n    Note/(a[0])"], &[]),
        (&[X, C, Q, "Osc o | freq=(q[0])"], &[]),
        (
            &[X, C, Q, "Osc o | freq=(q[0])", "Osc p | freq=(q[1])"],
            &[],
        ),
        (
            &[
                X,
                C,
                "let q = match c:\n    true: [x, 2.5]\n    false: [3, 4]",
                "Osc o | freq=(q[0])",
            ],
            &[],
        ),
        (
            &[
                X,
                C,
                "let q = if c then [[x, 2.5], [1, 2]] else [[3, 4], [5, 6]]",
                "Osc o | freq=(q[0][0])",
            ],
            &[],
        ),
        (
            &[
                X,
                C,
                "fn f(a): a[0]",
                "Osc o | freq=(f(if c then [x, 2.5] else [3, 4]))",
            ],
            &[],
        ),
        (
            &[
                X,
                C,
                "state y(2.5)",
                "let q = if c then [x, y] else [3, 4]",
                "Osc o | freq=(q[0])",
            ],
            &[],
        ),
        (
            &[
                X,
                C,
                "let q = if true then [x, 2.5] else [3, 4]",
                "Osc o | freq=(q[0])",
            ],
            &[],
        ),
        (
            &[X, C, Q, "Clip k:\n    Note/(q[1])"],
            &[(Code::FloatToInteger, "q[1]")],
        ),
        (&[X, C, "let a = [x, 2.5]", "Osc o | freq=(a[0])"], &[]),
        (
            &[
                X,
                C,
                "let q = if c then [x, 2] else [3, 4]",
                "Osc o | freq=(q[0])",
            ],
            &[],
        ),
        (
            &[X, C, "let q = if c then [x, \"s\"] else [3, 4]"],
            &[(Code::TypeMismatch, "[x, \"s\"]")],
        ),
    ];
    expect_in_orders(orders, &rows);
}

#[test]
fn a_cell_read_through_a_literals_group_keeps_its_own_kind() {
    a_cell_read_through_a_literals_group_keeps_its_own_kind_in(support::Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_cell_read_through_a_literals_group_keeps_its_own_kind() {
    a_cell_read_through_a_literals_group_keeps_its_own_kind_in(support::Orders::Every);
}

fn as_given(items: &[&'static str]) -> Vec<&'static str> {
    items.to_vec()
}

#[test]
fn a_family_feed_stays_out_of_an_index_and_a_plain_cell_conflicts_once() {
    const I: &str = "state i(0)";
    const X: &str = "state x(2)";
    const XA: &str = "let a = [1bars, x]";
    let rows: [LocatedRow; 10] = [
        (
            &[
                I,
                "let a = [1bars, 2bars]",
                "Bar b1 | bar_len=(a[i])",
                "Bar b2 | beat_len=(a[i])",
            ],
            &[],
        ),
        (
            &[
                I,
                "let a = [1bars, 2]",
                "Bar b1 | bar_len=(a[i])",
                "Bar b2 | beat_len=(a[i])",
            ],
            &[(Code::ConflictingUnits, "2", &["a[i]", "a[i]"])],
        ),
        (&[I, "let a = [1bars, 2]", "Bar b1 | bar_len=(a[i])"], &[]),
        (
            &[I, "let a = [1bars, 2]", "Bar b1 | bar_len=(a[i]) plain=i"],
            &[],
        ),
        (
            &[X, "Bar b1 | bar_len=x", "Bar b2 | beat_len=x"],
            &[(Code::ConflictingUnits, "x", &["x"])],
        ),
        (
            &[X, XA, "Bar b1 | bar_len=(a[0])", "Bar b2 | beat_len=(a[0])"],
            &[(Code::ConflictingUnits, "x", &["a[0]", "a[0]"])],
        ),
        (
            &[X, XA, "Bar b1 | bar_len=(a[1])", "Bar b2 | beat_len=(a[1])"],
            &[(Code::ConflictingUnits, "x", &["a[1]", "a[1]"])],
        ),
        (
            &[X, XA, "Bar b1 | bar_len=(a[0])", "Bar b2 | bar_len=(a[1])"],
            &[],
        ),
        (&[X, XA, "Bar b1 | bar_len=(a[1])"], &[]),
        (&[X, XA, "Bar b1 | bar_len=(a[0])"], &[]),
    ];
    expect_located_with(&rows, as_given, bar_errors);
}

#[test]
fn a_literals_family_value_reaches_its_reads_inside_a_recursive_body() {
    let rows: [Row; 7] = [
        (
            &[
                "fn f(n): if n < 1 then [1bars, 2][0] else f(n - 1)",
                "Clip k:\n    Note | length=(f(2))",
            ],
            &[],
        ),
        (
            &[
                "fn f(n):\n    let a = [1bars, n]\n    if n < 1 then a[0] else f(n - 1)",
                "Clip k:\n    Note | length=(f(2))",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n < 1 then [1bars, 2][1] else f(n - 1)",
                "Clip k:\n    Note | length=(f(2))",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n < 1 then [1bars, 2] else f(n - 1)",
                "Clip k:\n    Note | length=(f(1)[0])",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n < 1 then [1bars, 2] else f(n - 1)",
                "Clip k:\n    Note | length=(f(1)[1])",
            ],
            &[],
        ),
        (
            &[
                "fn f(n):\n    let a = [1bars, n]\n    if n < 1 then a[0] + a[1] else f(n - 1)",
                "Clip k:\n    Note | length=(f(2))",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n < 1 then [1bars, 2][0] else f(n - 1)",
                "Clip k:\n    Note/(f(2))",
            ],
            &[(Code::PlainInFamily, "2"), (Code::UnitMismatch, "f(2)")],
        ),
    ];
    expect_in_every_order(&rows);
}

#[test]
fn a_recursive_bodys_faulty_literal_holding_a_recursive_result_draws_one_diagnostic() {
    const AT_LITERAL: &[(Code, &str, &[&str])] =
        &[(Code::TypeMismatch, "[r(n - 1), \"s\"]", &["r(2)"])];
    let rows: [LocatedRow; 11] = [
        (
            &[
                "fn r(n):\n    let a = [r(n - 1), \"s\"]\n    if n < 1 then 0 else a[0]",
                "Clip c:\n    Note/(r(2))",
            ],
            AT_LITERAL,
        ),
        (
            &[
                "fn r(n):\n    let a = [r(n - 1), Wave::Saw]\n    if n < 1 then 0 else a[0]",
                "Clip c:\n    Note/(r(2))",
            ],
            &[(Code::TypeMismatch, "[r(n - 1), Wave::Saw]", &["r(2)"])],
        ),
        (
            &[
                "fn r(n): if n < 1 then [1, 2] else [r(n - 1)[0], \"s\"]",
                "Clip c:\n    Note/(r(2)[0])",
            ],
            &[(Code::TypeMismatch, "[r(n - 1)[0], \"s\"]", &["r(2)"])],
        ),
        (
            &[
                "fn r(n): if n < 1 then 0 else g(n)[0]",
                "fn g(n): if n > 0 then [r(n - 1), \"s\"] else [0, 0]",
                "Clip c:\n    Note/(r(2))",
            ],
            AT_LITERAL,
        ),
        (
            &[
                "fn r(n):\n    let a = [r(n - 1), \"s\"]\n    if n < 1 then [0, 0] else a",
                "Clip c:\n    Note/(r(2)[0])",
            ],
            AT_LITERAL,
        ),
        (
            &[
                "fn r(n):\n    let a = [r(n - 1), 2]\n    if n < 1 then 0 else a[0]",
                "Clip c:\n    Note/(r(2))",
            ],
            &[],
        ),
        (
            &[
                "fn r(n): if n < 1 then [1, 2] else [r(n - 1)[0], 3]",
                "Clip c:\n    Note/(r(2)[1])",
            ],
            &[],
        ),
        (
            &[
                "fn r(n): if n < 1 then 0 else g(n)[0]",
                "fn g(n): if n > 0 then [r(n - 1), 1] else [0, 0]",
                "Clip c:\n    Note/(r(2))",
            ],
            &[],
        ),
        (
            &[
                "fn r(n): if n < 1 then [1, 2] else r(n - 1)",
                "Clip c:\n    Note/(r(2)[0])",
            ],
            &[],
        ),
        (
            &[
                "fn r(n):\n    let a = [r(n - 1), \"s\"]\n    if n < 1 then 0 else a[1]",
                "Kit k:\n    Sampler | file=(r(2))",
            ],
            &[(Code::TypeMismatch, "if n < 1 then 0 else a[1]", &["r(2)"])],
        ),
        (
            &[
                "fn r(n): if n < 1 then 0 else [r(n - 1), \"s\"][1]",
                "Kit k:\n    Sampler | file=(r(2))",
            ],
            &[(
                Code::TypeMismatch,
                "if n < 1 then 0 else [r(n - 1), \"s\"][1]",
                &["r(2)"],
            )],
        ),
    ];
    expect_located_in_every_order(&rows);
}

fn no_secondary_repeats_the_primary(text: &str) -> bool {
    support::checked(text).iter().all(|diagnostic| {
        diagnostic
            .related
            .iter()
            .map(|related| &related.span)
            .all(|span| span.range != diagnostic.span.range)
    })
}

fn inconsistent_uses_name_each_use_with_its_calls_in(orders: support::Orders) {
    const IU: Code = Code::InconsistentUses;
    let rows: [LocatedRow; 13] = [
        (
            &[
                "let a = [2 ** -1]",
                "fn f(x): x[0]",
                "Clip c:\n    Note/(f(a)) | brightness=(f(a))",
            ],
            &[(IU, "x[0]", &["f(a)", "f(a)"])],
        ),
        (
            &[
                "let q = 2 ** -1",
                "fn f(): q",
                "Clip c:\n    Note/(f()) | brightness=(f())",
            ],
            &[(IU, "q", &["f()", "f()"])],
        ),
        (
            &[
                "let a = [2 ** -1]",
                "fn f(x): x[0]",
                "Clip c:\n    Note/(f(a))",
                "Clip d:\n    Note | brightness=(f(a))",
            ],
            &[(IU, "x[0]", &["f(a)", "f(a)"])],
        ),
        (
            &[
                "let a = [2 ** -1]",
                "fn g(y): y[0]",
                "fn f(x): g(x)",
                "Clip c:\n    Note/(f(a)) | brightness=(f(a))",
            ],
            &[(IU, "y[0]", &["g(x)", "f(a)", "g(x)", "f(a)"])],
        ),
        (
            &[
                "let a = [2 ** -1]",
                "fn f(x): x[0]",
                "Clip c:\n    Note/(f(a))\n    Note/(f(a))",
                "Clip d:\n    Note | brightness=(f(a))",
            ],
            &[(IU, "x[0]", &["f(a)", "f(a)"])],
        ),
        (
            &[
                "let a = [2 ** -1]",
                "fn f(): a[0]",
                "Clip c:\n    Note/(f()) | brightness=(f())",
            ],
            &[(IU, "a[0]", &["f()", "f()"])],
        ),
        (
            &[
                "let a = [2 ** -1]",
                "Clip c:\n    Note/(a[0]) | brightness=(a[0])",
            ],
            &[(IU, "a[0]", &["a[0]"])],
        ),
        (
            &["let q = 2 ** -1", "Clip c:\n    Note/(q) | brightness=q"],
            &[(IU, "q", &["q"])],
        ),
        (
            &[
                "let q = 2 ** -1",
                "fn f(x): x",
                "Clip c:\n    Note/(f(q)) | brightness=(f(q))",
            ],
            &[(IU, "q", &["q"])],
        ),
        (
            &[
                "let a = [2 ** -1]",
                "pattern P(x):\n    Note/(x[0]) | brightness=(x[0])",
                "Clip c:\n    P | x=a",
            ],
            &[(IU, "x[0]", &["x[0]", "P | x=a"])],
        ),
        (
            &[
                "let q = 2 ** -1",
                "pattern P():\n    Note/(q)",
                "Clip c:\n    P",
                "Clip d:\n    Note | brightness=q",
            ],
            &[(IU, "q", &["q", "P"])],
        ),
        (
            &[
                "pattern P(default n):\n    Note/(n) | velocity=n",
                "Clip c:\n    for e in [2 ** -1, 1]:\n        P/(e)",
            ],
            &[(IU, "n", &["n", "P/(e)"])],
        ),
        (
            &[
                "let x = 2 ** -1",
                "Clip c:\n    for e in [x, 1]:\n        Note/(e)\n        Note | velocity=e",
            ],
            &[(IU, "e", &["e"])],
        ),
    ];
    expect_located_in_orders(orders, &rows);
    let failures: Vec<String> = rows
        .iter()
        .filter_map(|(items, _)| {
            first_order_failing_in(
                orders,
                &with_session(items),
                &true,
                no_secondary_repeats_the_primary,
            )
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let text = "Session s\nlet a = [2 ** -1]\nfn f(x): x[0]\nClip c:\n    Note/(f(a)) | brightness=(f(a))\n";
    let calls: Vec<Vec<(u32, u32)>> = support::checked(text)
        .iter()
        .map(|diagnostic| {
            let mut ranges: Vec<(u32, u32)> = diagnostic
                .related
                .iter()
                .map(|related| &related.span)
                .map(|span| (span.range.start, span.range.end))
                .collect();
            ranges.sort();
            ranges
        })
        .collect();
    let mut both = vec![support::at(text, "f(a)", 0), support::at(text, "f(a)", 1)];
    both.sort();
    assert_eq!(calls, vec![both]);
}

#[test]
fn inconsistent_uses_name_each_use_with_its_calls() {
    inconsistent_uses_name_each_use_with_its_calls_in(support::Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_inconsistent_uses_name_each_use_with_its_calls() {
    inconsistent_uses_name_each_use_with_its_calls_in(support::Orders::Every);
}

#[test]
fn bytes_of_an_integer_array_keeps_its_shape_and_reads_as_a_u8_array() {
    let rows: &[support::Row] = &[
        (
            &["state xs([1, 2])\n", "Clip c:\n    Note/(xs.bytes[0])\n"],
            &[],
        ),
        (
            &[
                "state i(300)\n",
                "let b = [i, 2].bytes\n",
                "Clip c:\n    Note/(b[0])\n",
            ],
            &[],
        ),
        (
            &[
                "state xs([1, 2, 3]): u8[3]\n",
                "let b = xs.bytes\n",
                "Clip c:\n    Note/(b[2])\n",
            ],
            &[],
        ),
        (
            &[
                "state notes([60, 64, 67]): i32[3]\n",
                "let a = notes.bytes\n",
                "Clip c:\n    Note/(a[0])\n",
            ],
            &[],
        ),
        (
            &[
                "state i(1)\n",
                "let lit = [i, 2, 3]\n",
                "Clip c:\n    Note/(lit.bytes[1])\n",
            ],
            &[],
        ),
        (
            &[
                "state held([]): i32[..6]\n",
                "let raw = held.bytes\n",
                "let second = raw[1]\n",
                "Clip c:\n    Note/second\n",
            ],
            &[],
        ),
        (
            &[
                "state packet([]): u8[..6]\n",
                "state offset(0)\n",
                "pattern P(default v):\n    Note/v\n",
                "Clip c:\n    P/(packet.bytes[offset])\n",
            ],
            &[],
        ),
        (
            &[
                "state h([]): i32[..4]\n",
                "Clip c:\n    Note/(\"GET\".bytes[0] + [1, 2].bytes[1] + h.bytes.size)\n",
            ],
            &[],
        ),
        (
            &[
                "let packed = [300, -5, 70000, 12].bytes\n",
                "Clip c:\n    Note/(packed[0])\n    Note/(packed[1])\n",
            ],
            &[],
        ),
        (
            &["state xs([1, 2])\n", "Clip c:\n    Note/(xs.bytes[2])\n"],
            &[(Code::IndexOutOfRange, "2")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
    let attribute: &[support::Row] = &[
        (
            &[
                "state xs([1, 2, 3])\n",
                "Clip c | fixed=(xs.bytes):\n    Note\n",
            ],
            &[],
        ),
        (
            &[
                "state xs([1, 2, 3, 4])\n",
                "Clip c | fixed=(xs.bytes):\n    Note\n",
            ],
            &[(Code::TypeMismatch, "xs.bytes")],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), attribute);
}

#[test]
fn bytes_of_a_single_value_is_an_error() {
    let rows: &[support::Row] = &[
        (
            &["Clip c:\n    Note/((65).bytes[0])\n"],
            &[(Code::UnknownAttribute, "(65).bytes")],
        ),
        (
            &["let b = (65).bytes\n", "Clip c:\n    Note/b\n"],
            &[(Code::UnknownAttribute, "(65).bytes")],
        ),
        (
            &["Clip c | data=((65).bytes):\n    Note\n"],
            &[(Code::UnknownAttribute, "(65).bytes")],
        ),
        (
            &["state i(1)\n", "Clip c | data=(i.bytes):\n    Note\n"],
            &[(Code::UnknownAttribute, "i.bytes")],
        ),
        (
            &["Clip c | data=(\"GET\".bytes + [1, 2].bytes):\n    Note\n"],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), rows);
}

#[test]
fn a_fixed_u8_argument_takes_the_calls_type_through_a_recursive_body() {
    let call = "Clip c:\n    Note/(r(u8(3), 1))\n";
    let rows: &[support::Row] = &[
        (
            &[
                "fn r(x, n): if n < 1 then x - u8(5) else r(x, n - 1)\n",
                call,
            ],
            &[],
        ),
        (
            &["fn g(x): x - u8(5)\n", "Clip c:\n    Note/(g(u8(3)))\n"],
            &[],
        ),
        (
            &["fn r(x, n): if n < 1 then x + 300 else r(x, n - 1)\n", call],
            &[],
        ),
        (
            &[
                "fn r(x, n): if n < 1 then x + u8(1) else r(x, n - 1)\n",
                call,
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_literal_beside_a_fixed_type_operand_takes_the_flowing_type() {
    let rows: &[support::Row] = &[
        (&["Clip c:\n    Note/(u8(250) * 2 - 400)\n"], &[]),
        (&["Clip c:\n    Note/(u8(250) - 400)\n"], &[]),
        (&["Clip c:\n    Note/(u8(100) * 2 - 400)\n"], &[]),
        (&["Clip c:\n    Note/(u8(200) + 100)\n"], &[]),
        (
            &["Clip c:\n    Note when u8(250) < 400\n"],
            &[(Code::DoesNotFit, "400")],
        ),
        (
            &["Clip c:\n    Note/([1, 2, 3][u8(2) + 300])\n"],
            &[(Code::DoesNotFit, "300")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_fixed_type_operand_widens_into_the_expected_type_before_the_operation() {
    let rows: &[support::Row] = &[
        (&["Clip c:\n    Note/(u8(255) + 4)\n"], &[]),
        (&["Clip c:\n    Note/(u8(250) + 300)\n"], &[]),
        (
            &["Clip c:\n    Note | level=(u8(255) + 4)\n"],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
        (
            &["Clip c:\n    Note when u8(255) + 4 > 0\n"],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
        (
            &["Clip c:\n    Note when u8(255) + 4 > 1i32\n"],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
        (
            &["let k = u8(255) + 4\n", "Clip c:\n    Note when k > 0\n"],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
        (
            &["fn g(x): x - u8(5)\n", "Clip c:\n    Note/(g(u8(3)))\n"],
            &[],
        ),
        (
            &[
                "fn g(x): x - u8(5)\n",
                "Clip c:\n    Note | level=(g(u8(3)))\n",
            ],
            &[(Code::ConstantOverflow, "x - u8(5)")],
        ),
        (
            &[
                "let packed = [300, -5, 70000, 12].bytes\n",
                "Clip c:\n    Note/(packed[0] + packed.size)\n",
            ],
            &[],
        ),
        (
            &["Clip c:\n    Note/(2147483647i32 + 1)\n"],
            &[(Code::ConstantOverflow, "2147483647i32 + 1")],
        ),
        (&["Clip c:\n    Note/(-u8(3))\n"], &[]),
        (
            &["Clip c:\n    Note | level=(-u8(3))\n"],
            &[(Code::ConstantOverflow, "-u8(3)")],
        ),
        (
            &["Clip c:\n    Note/(if true then u8(255) + 4 else 0)\n"],
            &[],
        ),
        (
            &["Clip c:\n    Note | level=(if true then u8(255) + 4 else 0)\n"],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
        (
            &[
                "let m = match 1:\n    1: u8(255) + 4\n    else: 0\n",
                "Clip c:\n    Note/m\n",
            ],
            &[],
        ),
        (
            &["let xs = [u8(255) + 4, 1]\n", "Clip c:\n    Note/(xs[0])\n"],
            &[],
        ),
        (
            &[
                "let xs = [u8(255) + 4, 1]\n",
                "Clip c:\n    Note | level=(xs[0])\n",
            ],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
        (&["Clip c:\n    Note/(max(u8(255) + 4, 0))\n"], &[]),
        (
            &["Clip c:\n    Note | level=(max(u8(255) + 4, 0))\n"],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
        (&["Clip c:\n    Note/((u8(255) + 4) div 2)\n"], &[]),
        (&["Clip c:\n    Note/(plain(u8(255) + 4))\n"], &[]),
        (
            &["Clip c | fixed=[u8(255) + 4, 0, 0]:\n    Note\n"],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), rows);
    let frequencies: &[support::Row] = &[
        (&["Osc o | freq=(2147483647i32 + 1)\n"], &[]),
        (&["Osc o | freq=(3.4028235e38f32 * 2)\n"], &[]),
        (&["Osc o | freq=(u8(250) * 2 / 2)\n"], &[]),
        (&["Osc o | freq=(sqrt(u8(250) * 2))\n"], &[]),
        (&["Osc o | freq=(u8(250) / 300)\n"], &[]),
        (
            &[
                "fn r(x, n): if n < 1 then x else r(x, n - 1)\n",
                "Osc o | freq=(r(u8(250) * 2 / 2, 1))\n",
            ],
            &[(Code::ConstantOverflow, "u8(250) * 2")],
        ),
        (
            &["Osc o | freq=((u8(255) + 4) div 2)\n"],
            &[(Code::ConstantOverflow, "u8(255) + 4")],
        ),
        (
            &[
                "let r = u8(250) * 2 / 2\n",
                "Clip c:\n    Note when r > 1\n",
            ],
            &[(Code::ConstantOverflow, "u8(250) * 2")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), frequencies);
}

#[test]
fn a_comparisons_operand_computes_in_its_own_types_before_it_takes_its_partners() {
    let big = "state big(1)\n";
    let flag = "state flag(false)\n";
    let u8_sum: &[(Code, &str)] = &[(Code::ConstantOverflow, "u8(255) + 4")];
    let i32_sum: &[(Code, &str)] = &[(Code::ConstantOverflow, "2147483647i32 + 1")];
    let i32_product: &[(Code, &str)] = &[(Code::ConstantOverflow, "2147483647i32 * 2")];
    let rows: &[support::Row] = &[
        (
            &[big, "Clip c:\n    Note when big > 2147483647i32 + 1\n"],
            i32_sum,
        ),
        (&[big, "Clip c:\n    Note when big > u8(255) + 4\n"], u8_sum),
        (&[big, "Clip c:\n    Note when u8(255) + 4 < big\n"], u8_sum),
        (
            &[
                "state k(1)\n",
                "Clip c:\n    Note when k < 2147483647i32 * 2\n",
            ],
            i32_product,
        ),
        (
            &[
                "state k(1)\n",
                "let r = k > u8(255) + 4\n",
                "Clip c:\n    Note when r\n",
            ],
            u8_sum,
        ),
        (
            &[big, "Clip c:\n    Note when big >= u8(255) + 4\n"],
            u8_sum,
        ),
        (
            &[big, "Clip c:\n    Note when u8(255) + 4 <= big\n"],
            u8_sum,
        ),
        (
            &[big, "Clip c:\n    Note when big is u8(255) + 4\n"],
            u8_sum,
        ),
        (
            &[big, "Clip c:\n    Note when u8(255) + 4 is not big\n"],
            u8_sum,
        ),
        (
            &[
                big,
                "Clip c:\n    Note/(if big > u8(255) + 4 then 60 else 62)\n",
            ],
            u8_sum,
        ),
        (
            &[
                big,
                "Clip c:\n    when:\n        big > u8(255) + 4: Note/60\n        else: Note/62\n",
            ],
            u8_sum,
        ),
        (
            &[
                big,
                "Clip c:\n    when u8(255) + 4 < big:\n        Note/60\n",
            ],
            u8_sum,
        ),
        (
            &[
                big,
                "fn over(x): x > u8(255) + 4\n",
                "Clip c:\n    Note when over(big)\n",
            ],
            u8_sum,
        ),
        (
            &[
                big,
                "pattern Gated(default active):\n    Note when active\n",
                "Clip c:\n    Gated/(big > u8(255) + 4)\n",
            ],
            u8_sum,
        ),
        (
            &[
                big,
                flag,
                "Clip c:\n    Note when big > (if flag then u8(255) + 4 else 0)\n",
            ],
            u8_sum,
        ),
        (
            &["Clip c:\n    Note when 1i64 > 2147483647i32 + 1\n"],
            i32_sum,
        ),
        (&["Clip c:\n    Note when u8(255) + 4 > 1i32\n"], u8_sum),
        (
            &["state k(1)\n", "Clip c:\n    Note/(k + (u8(255) + 4))\n"],
            &[],
        ),
        (&[big, "Clip c:\n    Note when big > u8(250) + 4\n"], &[]),
        (&[big, "Clip c:\n    Note when big > 2147483647 + 1\n"], &[]),
        (
            &[
                big,
                flag,
                "Clip c:\n    Note when big > (if flag then u8(250) + 4 else 300)\n",
            ],
            &[(Code::DoesNotFit, "300")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn where_the_flow_stops_an_operand_computes_in_its_own_types_whatever_its_partner() {
    let k = "state k(1)\n";
    let flag = "state flag(false)\n";
    let u8_sum: &[(Code, &str)] = &[(Code::ConstantOverflow, "u8(255) + 4")];
    let i32_sum: &[(Code, &str)] = &[(Code::ConstantOverflow, "2147483647i32 + 1")];
    let summed = "let r = k + (u8(255) + 4)\n";
    let rows: &[support::Row] = &[
        (
            &[k, "Clip c:\n    Note when k + (u8(255) + 4) > 0\n"],
            u8_sum,
        ),
        (
            &[k, "Clip c:\n    Note when k + (2147483647i32 + 1) > 0\n"],
            i32_sum,
        ),
        (&[k, summed, "Clip c:\n    Note when r > 0\n"], u8_sum),
        (
            &[k, "Clip c:\n    Note/([60, 62][k + (u8(255) + 4)])\n"],
            u8_sum,
        ),
        (
            &[k, "Synth s:\n    Osc | freq=((k + (u8(255) + 4)) div 2)\n"],
            u8_sum,
        ),
        (
            &[k, "Clip c:\n    Note when 0 < k + (u8(255) + 4)\n"],
            u8_sum,
        ),
        (
            &[
                k,
                "state held([]): i32[..4]\n",
                "Clip c:\n    Note/(held[k + (u8(255) + 4)])\n",
            ],
            u8_sum,
        ),
        (&[k, summed, "Clip c:\n    Note/([60, 62][r])\n"], u8_sum),
        (
            &[
                k,
                "fn g(x): x + (u8(255) + 4)\n",
                "Clip c:\n    Note when g(k) > 0\n",
            ],
            u8_sum,
        ),
        (
            &[
                k,
                "pattern Gated(default v):\n    Note when v > 0\n",
                "Clip c:\n    Gated/(k + (u8(255) + 4))\n",
            ],
            u8_sum,
        ),
        (
            &[
                k,
                flag,
                "Clip c:\n    Note when (if flag then k + (u8(255) + 4) else k) > 0\n",
            ],
            u8_sum,
        ),
        (
            &[k, "Clip c:\n    Note when 1i64 + (u8(255) + 4) > 0\n"],
            u8_sum,
        ),
        (&[k, "Clip c:\n    Note/((k + (u8(255) + 4)) div 2)\n"], &[]),
        (&[k, summed, "Clip c:\n    Note/r\n"], &[]),
        (&[k, "Clip c:\n    Note when k + (u8(250) + 4) > 0\n"], &[]),
        (
            &[k, "Clip c:\n    Note when k + (2147483647 + 1) > 0\n"],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn an_integer_inputs_initial_value_fits_every_use_u8_uses_included() {
    let rows: &[support::Row] = &[
        (
            &["state i(300)\n", "Clip c | fixed=[i, 2, 3]:\n    Note\n"],
            &[(Code::DoesNotFit, "i")],
        ),
        (
            &["state i(300)\n", "let x: u8 = i\n", "Clip c:\n    Note/x\n"],
            &[(Code::DoesNotFit, "i")],
        ),
        (
            &["state i(300)\n", "Clip c:\n    Note/(fixed3([i, 1, 2]))\n"],
            &[(Code::DoesNotFit, "i")],
        ),
        (
            &[
                "state xs([300]): i64[..4]\n",
                "Clip c | data=xs:\n    Note\n",
            ],
            &[(Code::DoesNotFit, "xs")],
        ),
        (
            &[
                "state xs([3000000000]): i64[..4]\n",
                "Clip c:\n    Note/(xs[0])\n",
            ],
            &[(Code::DoesNotFit, "xs")],
        ),
        (
            &[
                "state i(255)\n",
                "state xs([255]): i64[..4]\n",
                "let x: u8 = i\n",
                "Clip c | fixed=[i, 2, 3] data=xs:\n    Note/x\n    Note/(fixed3([i, 1, 2]))\n",
            ],
            &[],
        ),
        (
            &[
                "state i(300)\n",
                "let b = [i, 2].bytes\n",
                "Clip c:\n    Note/(b[0])\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), rows);
}

#[test]
fn a_match_label_with_the_scrutinees_type_suffix_is_accepted() {
    let read = "Clip c:\n    Note/v\n";
    let rows: &[support::Row] = &[
        (
            &[
                "state b(3): u8\n",
                "let v = match b:\n    3u8: 1\n    else: 2\n",
                read,
            ],
            &[],
        ),
        (
            &[
                "state b(3): u8\n",
                "let v = match b:\n    3: 1\n    else: 2\n",
                read,
            ],
            &[],
        ),
        (
            &[
                "state b(3): u8\n",
                "let v = match b:\n    3i32: 1\n    else: 2\n",
                read,
            ],
            &[(Code::MatchLabel, "3i32")],
        ),
        (
            &[
                "state b(3)\n",
                "let v = match b:\n    3i64: 1\n    else: 2\n",
                read,
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
    let compiler =
        symphonyscript::Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("compiles");
    let text = "state b(3): u8\nlet v = match b:\n    3i32: 1\n    else: 2\nClip c:\n    Note/v\n";
    let diagnostics = compiler.check(&[support::source(text)]);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == Code::MatchLabel
                && diagnostic.summary.contains("`u8`")),
        "{diagnostics:?}"
    );
}

#[test]
fn a_match_label_is_fit_checked_in_the_scrutinees_type_and_in_its_suffixs() {
    let byte = "state b(3): u8\n";
    let int = "state b(3): i32\n";
    let wide = "state b(3)\n";
    let read = "Clip c:\n    Note/(v)\n";
    let labelled = |scrutinee: &str, label: &str| {
        format!("let v = match {scrutinee}:\n    {label}: 1\n    else: 2\n")
    };
    let suffixed = labelled("b", "300u8");
    let past = labelled("b", "300");
    let negative = labelled("b", "-1");
    let past_i32 = labelled("b", "3000000000");
    let past_suffixed = labelled("b", "3000000000i32");
    let past_i64 = labelled("b", "9223372036854775808");
    let through_let = labelled("s", "300");
    let top = labelled("b", "255u8");
    let top_flexible = labelled("b", "255");
    let zero = labelled("b", "0");
    let other = labelled("b", "3i32");
    let rows: &[support::Row] = &[
        (&[byte, &suffixed, read], &[(Code::DoesNotFit, "300u8")]),
        (&[byte, &past, read], &[(Code::DoesNotFit, "300")]),
        (&[byte, &negative, read], &[(Code::DoesNotFit, "-1")]),
        (&[int, &past_i32, read], &[(Code::DoesNotFit, "3000000000")]),
        (
            &[int, &past_suffixed, read],
            &[(Code::DoesNotFit, "3000000000i32")],
        ),
        (
            &[wide, &past_i64, read],
            &[(Code::DoesNotFit, "9223372036854775808")],
        ),
        (
            &["let b = u8(3)\n", &past, read],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &[byte, "let s = b\n", &through_let, read],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &[
                byte,
                "fn pick(x):\n    match x:\n        300: 1\n        else: 2\n",
                "Clip c:\n    Note/(pick(b))\n",
            ],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &[
                byte,
                "Clip a:\n    Note\n    Plays: match b:\n        300: a\n        else: a\n",
            ],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &[
                "let b = u8(3)\n",
                "Clip a:\n    Note\n    Plays: match b:\n        300: a\n        else: a\n",
            ],
            &[(Code::DoesNotFit, "300")],
        ),
        (&[byte, &top, read], &[]),
        (&[byte, &top_flexible, read], &[]),
        (&[byte, &zero, read], &[]),
        (&[wide, &past_i32, read], &[]),
        (&[byte, &other, read], &[(Code::MatchLabel, "3i32")]),
        (
            &[
                "state wave(Wave::Saw)\n",
                "let level = match wave:\n    Sine: 1.0\n    Saw: 0.8\n    else: 0.6\n",
                "Clip c | gain=level:\n    Note\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn an_annotation_is_its_initializers_expected_type() {
    let rows: &[support::Row] = &[
        (&["state w(Saw): Wave\n", "Osc o | wave=w\n"], &[]),
        (&["let x: Wave = Saw\n", "Osc o | wave=x\n"], &[]),
        (&["state s: Wave = Saw\n"], &[]),
        (
            &["state w([Saw, Sine]): Wave[2]\n", "Osc o | wave=(w[1])\n"],
            &[],
        ),
        (&["state w(Saw)\n"], &[(Code::UnknownName, "Saw")]),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn one_fault_draws_one_diagnostic_at_the_fault() {
    let note = "Clip c:\n    Note\n";
    support::assert_rows_in_every_order(
        &support::signature_vocabulary(),
        &[
            (
                &["state f(1.5)\n", "Clip c | three=[f, 2]:\n    Note\n"],
                &[(Code::FloatToInteger, "f")],
            ),
            (
                &["state f(1.5)\n", "Clip c | three=[1.5, 2]:\n    Note\n"],
                &[(Code::TypeMismatch, "1.5")],
            ),
            (
                &["state h([]): u8[..3]\n", "Clip c | fixed=h:\n    Note\n"],
                &[(Code::VariableArray, "h")],
            ),
        ],
    );
    let state = "state x(1)\n";
    support::assert_rows_in_every_order(
        &symphonyscript_fixtures::vocabulary(),
        &[
            (
                &[
                    "fn f(x):\n    state s = 1\n    x\n",
                    "Clip c:\n    Note/(f(1))\n",
                ],
                &[(Code::CellSyntax, "state")],
            ),
            (
                &[
                    "fn f(x):\n    let y = 1\n    state s = 1\n    x + y\n",
                    "Clip c:\n    Note/(f(1))\n",
                ],
                &[(Code::CellSyntax, "state")],
            ),
            (
                &[
                    "pattern P():\n    state s = 1\n    Note\n",
                    "Clip c:\n    P\n",
                ],
                &[(Code::UnbuiltConstruct, "state s = 1")],
            ),
            (
                &[
                    state,
                    "state s = x.past(1, 0.5)\n",
                    "Clip c:\n    Note when s > 0\n",
                ],
                &[(Code::TypeMismatch, "0.5")],
            ),
            (&["state f(1.5)\n", "Osc o | freq=f\n", note], &[]),
        ],
    );
    let text = "state x(1)\nstate s = x.past(1, 0.5)\nClip c:\n    Note when s > 0\n";
    assert!(
        support::checked_all(text)
            .iter()
            .all(|diagnostic| diagnostic.code != Code::IntegerCell),
        "{text}"
    );
}

#[test]
fn a_choice_holding_an_operand_a_syntax_fault_leaves_missing_draws_its_syntax_diagnostic_alone() {
    let inputs = "state l(1)\nstate k([1, 2]): i32[..4]\n";
    let positions: [(&str, &str); 3] = [
        ("let j = ", "\nClip c:\n    Note/(j)\n"),
        ("state j = ", "\nClip c:\n    Note/(j)\n"),
        ("Clip c:\n    Note/(", ")\n"),
    ];
    let mut choices = Vec::new();
    for condition in ["5", "l", "k[0]"] {
        choices.push((
            format!("if {condition}] > 1 then 1 else [3]"),
            format!("if {condition} > 1 then 1 else [3]"),
            Code::TypeMismatch,
        ));
        choices.push((
            format!("if {condition} then else 2"),
            format!("if {condition} then 1 else 2"),
            Code::ConditionType,
        ));
        choices.push((
            format!("if {condition} then 1 else"),
            format!("if {condition} then 1 else 2"),
            Code::ConditionType,
        ));
    }
    choices.push((
        "if then 1 else [3]".to_string(),
        "if l > 1 then 1 else [3]".to_string(),
        Code::TypeMismatch,
    ));
    let mut failures = Vec::new();
    for (before, after) in positions {
        for (faulty, twin, drawn) in &choices {
            let text = format!("{inputs}{before}{faulty}{after}");
            let found = codes(&text);
            if found.is_empty() || found.iter().any(|code| *code != Code::ExpressionSyntax) {
                failures.push(format!(
                    "{text:?}: expected the syntax diagnostic alone, found {found:?}"
                ));
            }
            let text = format!("{inputs}{before}{twin}{after}");
            let found = codes(&text);
            if found != [*drawn] {
                failures.push(format!("{text:?}: expected {drawn:?}, found {found:?}"));
            }
        }
    }
    let envelope = symphonyscript_fixtures::envelope_vocabulary();
    for (value, drawn) in [
        ("p + ", Code::ExpressionSyntax),
        ("p[ + 1s", Code::ExpressionSyntax),
        ("if p then 1s else", Code::ExpressionSyntax),
        ("if p then 1s else 2s", Code::ConditionType),
    ] {
        let text = format!("let p = 3\nEnv e | attack=({value}) pitch=p\n");
        let found: Vec<Code> = support::checked_with(&envelope, &text)
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        if found != [drawn] {
            failures.push(format!("{text:?}: expected {drawn:?}, found {found:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_bytes_result_of_another_shape_than_a_fixed_attributes_draws_the_shape_diagnostic_alone() {
    let binding = "Clip c | fixed=(xs.bytes):\n    Note\n";
    support::assert_rows_in_every_order(
        &support::signature_vocabulary(),
        &[
            (
                &["state xs([1, 2]): u8[2]\n", binding],
                &[(Code::TypeMismatch, "xs.bytes")],
            ),
            (
                &["state xs([1, 2])\n", binding],
                &[(Code::TypeMismatch, "xs.bytes")],
            ),
            (&["state xs([1, 2, 3])\n", binding], &[]),
        ],
    );
}

#[test]
fn an_input_whose_initializer_does_not_fit_its_annotation_draws_one_does_not_fit_at_the_value() {
    support::assert_rows_in_every_order(
        &support::signature_vocabulary(),
        &[
            (
                &[
                    "state xs([300]): u8[..3]\n",
                    "Clip c | data=xs:\n    Note\n",
                ],
                &[(Code::DoesNotFit, "300")],
            ),
            (
                &[
                    "state xs([3000000000]): i32[..3]\n",
                    "Clip c:\n    Note/(xs[0])\n",
                ],
                &[(Code::DoesNotFit, "3000000000")],
            ),
            (
                &["state x(2147483647 + 1): i32\n", "Clip c:\n    Note/x\n"],
                &[(Code::DoesNotFit, "2147483647 + 1")],
            ),
            (
                &[
                    "state xs([300, 1, 2]): u8[3]\n",
                    "Clip c | fixed=xs:\n    Note\n",
                ],
                &[(Code::DoesNotFit, "300")],
            ),
            (
                &["state i(300)\n", "Clip c | fixed=[i, 2, 3]:\n    Note\n"],
                &[(Code::DoesNotFit, "i")],
            ),
        ],
    );
}

#[test]
fn a_bare_variant_in_a_for_iterable_draws_one_unknown_name_and_is_a_placeholder() {
    let rows: &[support::Row] = &[
        (
            &["Clip c:\n    for i in [Saw, 1]:\n        Note\n"],
            &[(Code::UnknownName, "Saw")],
        ),
        (
            &["Clip c:\n    for i in [1, Saw]:\n        Note\n"],
            &[(Code::UnknownName, "Saw")],
        ),
        (
            &["Clip c:\n    Note/([Saw, 1][0])\n"],
            &[(Code::TypeMismatch, "[Saw, 1]")],
        ),
        (
            &["let a = [1, Saw]\n", "Clip c:\n    Note/(a[0])\n"],
            &[(Code::TypeMismatch, "[1, Saw]")],
        ),
        (
            &["Synth s:\n    for w in [Wave::Saw, Wave::Sine]:\n        Osc | wave=w\n"],
            &[],
        ),
        (&["Clip c:\n    for i in [1, 2]:\n        Note\n"], &[]),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_range_bound_or_index_reading_a_diagnosed_let_takes_its_placeholder() {
    let mismatch = (Code::TypeMismatch, "Saw");
    let rows: &[support::Row] = &[
        (
            &[
                "let x = Saw\n",
                "Clip c:\n    for t in 0..x:\n        Note\n",
            ],
            &[mismatch],
        ),
        (
            &["let x = Saw\n", "Clip c:\n    Note/([1, 2][x])\n"],
            &[mismatch],
        ),
        (
            &[
                "let x = Saw\n",
                "pattern P(n):\n    for t in 0..n:\n        Note\n",
                "Clip c:\n    P | n=x\n",
            ],
            &[mismatch],
        ),
        (&["let x = Saw\n", "Clip c:\n    Note/x\n"], &[mismatch]),
        (
            &[
                "let x = Saw\n",
                "Clip c:\n    for t in [x, x]:\n        Note\n",
            ],
            &[mismatch],
        ),
        (
            &["Clip c:\n    for t in 0..Saw:\n        Note\n"],
            &[(Code::RangeBound, "Saw")],
        ),
        (
            &["let n = 3\n", "Clip c:\n    for t in 0..n:\n        Note\n"],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_to_path_reads_an_array_attribute_as_its_binding() {
    let declared = [
        "state sec(0)\n",
        "Clip b:\n    Note\n",
        "Clip c:\n    Note\n",
    ];
    let reads = "Clip a:\n    Plays | gain=to.steps[0]: b\n    Plays | gain=to.steps[1]: [b, c][sec]\n    Plays | gain=to.seq.size: b\n    Plays | gain=to.steps.size: [b, c][sec]\n";
    let lane = "Lane l\n";
    let rows: &[support::Row] = &[
        (&[declared[0], declared[1], declared[2], reads], &[]),
        (
            &[declared[1], "Clip a:\n    Plays | gain=to.seq[0]: b\n"],
            &[(Code::IndexOutOfRange, "0")],
        ),
        (
            &[declared[1], "Clip a:\n    Plays | gain=to.steps: b\n"],
            &[(Code::TypeMismatch, "to.steps")],
        ),
        (
            &[declared[1], "Clip a:\n    Plays | gain=b.steps: b\n"],
            &[(Code::TypeMismatch, "b.steps")],
        ),
        (
            &[
                declared[0],
                declared[1],
                lane,
                "Clip a:\n    Cues | gain=to.steps[0]: [b, l][sec]\n",
            ],
            &[(Code::ToCandidates, "to.steps")],
        ),
        (
            &[
                declared[0],
                declared[1],
                lane,
                "Clip a:\n    Cues | gain=to.seq[0]: [b, l][sec]\n",
            ],
            &[(Code::ToCandidates, "to.seq")],
        ),
        (
            &[
                declared[0],
                declared[1],
                declared[2],
                "Clip a:\n    Plays | gain=to.seq[1] + to.seq.size + to.steps[1..][0]: [b, c][sec]\n",
            ],
            &[],
        ),
        (
            &[
                declared[0],
                declared[1],
                declared[2],
                "Clip a:\n    Plays | trio=to.steps: [b, c][sec]\n    Plays | picks=to.seq[..2]: [b, c][sec]\n    Plays | gain=f32((to.seq + to.seq).size): [b, c][sec]\n",
            ],
            &[],
        ),
        (
            &[
                declared[0],
                declared[1],
                declared[2],
                "Clip a:\n    Plays | picks=to.seq: [b, c][sec]\n",
            ],
            &[(Code::CapacityExceeded, "to.seq")],
        ),
        (
            &[
                declared[0],
                declared[1],
                declared[2],
                "Clip a:\n    Plays | picks=to.seq + to.seq: [b, c][sec]\n",
            ],
            &[(Code::CapacityExceeded, "to.seq + to.seq")],
        ),
        (
            &[declared[1], "Clip a:\n    Plays | picks=to.seq: b\n"],
            &[],
        ),
        (
            &[
                "Clip b | seq=[1, 2, 3]:\n    Note\n",
                "Clip a:\n    Plays | picks=to.seq: b\n",
            ],
            &[(Code::CapacityExceeded, "to.seq")],
        ),
        (&[declared[1], "Clip a:\n    Plays | picks=b.seq: b\n"], &[]),
        (
            &[
                "Clip b | seq=[1, 2, 3]:\n    Note\n",
                "Clip a:\n    Plays | picks=b.seq: b\n",
            ],
            &[(Code::CapacityExceeded, "b.seq")],
        ),
        (
            &[
                declared[0],
                declared[1],
                declared[2],
                "Clip a:\n    Plays | trio=to.seq: [b, c][sec]\n",
            ],
            &[(Code::VariableArray, "to.seq")],
        ),
    ];
    support::assert_rows_in_every_order(&support::to_array_vocabulary(), rows);
}

#[test]
fn an_independent_fault_beside_a_placeholder_draws_its_own_diagnostic() {
    let rows: &[support::Row] = &[
        (
            &["Clip c:\n    Note/(nothing + (1 + true))\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::OperandType, "1 + true"),
            ],
        ),
        (
            &["Clip c:\n    Note/(nothing + [1, 2][5])\n"],
            &[(Code::UnknownName, "nothing"), (Code::IndexOutOfRange, "5")],
        ),
        (
            &["Clip c:\n    Note/(nothing + 300000000000)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["Clip c:\n    Note/(nothing + i32(true))\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::ConversionType, "i32(true)"),
            ],
        ),
        (
            &[
                "let y: i32[2] = [1, 2, 3]\n",
                "Clip c:\n    Note/([1, 2][5] + y[0])\n",
            ],
            &[
                (Code::TypeMismatch, "let y: i32[2] = [1, 2, 3]\n"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &[
                "let y: u8[..2] = \"GET\"\n",
                "Clip c:\n    Note/([1, 2][5] + i32(y[0]))\n",
            ],
            &[
                (Code::CapacityExceeded, "let y: u8[..2] = \"GET\"\n"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note/((nothing + 1) + (1 + true))\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::OperandType, "1 + true"),
            ],
        ),
        (
            &["Clip c:\n    Note/((1 + true) + [1, 2][5])\n"],
            &[
                (Code::OperandType, "1 + true"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note/([nothing, 300000000000][0])\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["state x(1)\n", "Clip c:\n    Note/([x, 300000000000][0])\n"],
            &[],
        ),
        (&["Clip c:\n    Note/([1, 300000000000][0])\n"], &[]),
        (
            &["Clip c:\n    Note/(if nothing then 1 + true else 2)\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::OperandType, "1 + true"),
            ],
        ),
        (
            &["Clip c:\n    Note/(if false then nothing else 300000000000)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["Clip c:\n    Note/(if true then nothing else 300000000000)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["Clip c:\n    Note when nothing and (1 + true)\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::OperandType, "1 + true"),
            ],
        ),
        (
            &["Clip c:\n    Note | length=(nothing + 2s)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[
                "fn g(a): a + 300000000000\n",
                "Clip c:\n    Note/(g(nothing))\n",
            ],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[
                "state note(60)\n",
                "state s = nothing + (1 + true)\n",
                "Clip c:\n    Note/(s + note)\n",
            ],
            &[
                (Code::UnknownName, "nothing"),
                (Code::OperandType, "1 + true"),
            ],
        ),
        (
            &["Clip c:\n    Note/(nothing + 1)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["Clip c:\n    Note/(nothing[5] + 1)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[
                "let y: i32[2] = [1, 2, 3]\n",
                "Clip c:\n    Note/(y[0] + 1)\n",
            ],
            &[(Code::TypeMismatch, "let y: i32[2] = [1, 2, 3]\n")],
        ),
        (
            &["let y: i32[2] = [1, 2, 3]\n", "Clip c:\n    Note/(y[5])\n"],
            &[(Code::TypeMismatch, "let y: i32[2] = [1, 2, 3]\n")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_runtime_choice_between_strings_where_a_u8_array_is_expected_is_a_variable_array_of_their_bytes()
 {
    let f = "state f(false)\n";
    let rows: &[support::Row] = &[
        (
            &[f, "Clip as g | data=(if f then \"GET\" else \"AB\")\n"],
            &[],
        ),
        (
            &[
                f,
                "Clip as g | data=(if f then \"GET\" else \"ABCDEFGHI\")\n",
            ],
            &[(
                Code::CapacityExceeded,
                "if f then \"GET\" else \"ABCDEFGHI\"",
            )],
        ),
        (
            &[
                f,
                "fn m(x):\n    match x:\n        true: \"GET\"\n        else: \"AB\"\n",
                "Clip as g | data=m(f)\n",
            ],
            &[],
        ),
        (
            &[f, "Clip as g | data=(if true then \"AB\" else \"CD\")\n"],
            &[],
        ),
        (&["Clip as g | data=\"GET\"\n"], &[]),
        (
            &[
                f,
                "Clip as g | data=(if f then [71, 69, 84] else [65, 66])\n",
            ],
            &[],
        ),
        (
            &[
                f,
                "let s = \"GET\"\n",
                "Clip as g | data=(if f then s else (if f then s else \"AB\"))\n",
            ],
            &[],
        ),
        (
            &[
                f,
                "let d: u8[..8] = if f then \"GET\" else \"AB\"\n",
                "Clip as g | data=d\n",
            ],
            &[],
        ),
        (
            &[
                f,
                "let d: u8[..2] = if f then \"GET\" else \"AB\"\n",
                "Clip as g | data=d\n",
            ],
            &[(
                Code::CapacityExceeded,
                "let d: u8[..2] = if f then \"GET\" else \"AB\"\n",
            )],
        ),
        (
            &[
                f,
                "state d: u8[..8] = if f then \"GET\" else \"AB\"\n",
                "Clip as g | data=d\n",
            ],
            &[],
        ),
        (
            &[f, "Clip as g | fixed=(if f then \"GET\" else \"ABC\")\n"],
            &[],
        ),
        (
            &[f, "Clip as g | fixed=(if f then \"GET\" else \"AB\")\n"],
            &[(Code::VariableArray, "if f then \"GET\" else \"AB\"")],
        ),
        (
            &[
                f,
                "state n(1)\n",
                "Clip as g | data=(if f then \"GET\" else \"A{n}\")\n",
            ],
            &[(Code::ReactiveString, "n")],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), rows);
    let compiler =
        symphonyscript::Compiler::new(&support::signature_vocabulary()).expect("it compiles");
    let text = "state f(false)\nClip as g | data=(if f then \"GET\" else \"ABCDEFGHI\")\n";
    let summaries: Vec<String> = support::errors(compiler.check(&[support::source(text)]))
        .into_iter()
        .map(|diagnostic| diagnostic.summary)
        .collect();
    assert_eq!(
        summaries,
        ["this value holds up to 9 elements, more than the attribute's capacity 8"],
        "{text}"
    );
    let rows: &[support::Row] = &[
        (
            &[
                f,
                "Kit k:\n    Sampler | file=(if f then \"a\" else \"b\")\n",
            ],
            &[(Code::ReactiveString, "if f then \"a\" else \"b\"")],
        ),
        (
            &[f, "Track t | label=(if f then \"x\" else \"y\")\n"],
            &[(Code::ReactiveString, "if f then \"x\" else \"y\"")],
        ),
        (
            &["Kit k:\n    Sampler | file=(if true then \"a\" else \"b\")\n"],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
    let rows: &[support::Row] = &[(
        &[
            f,
            "Song s:\n    Clip as c:\n        Note/(checksum(if f then \"a\" else \"bc\"))\n",
        ],
        &[],
    )];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::native_vocabulary(), rows);
}

#[test]
fn where_a_u8_array_is_expected_each_string_leaf_of_a_choice_is_its_bytes_whatever_the_other_leaves_are()
 {
    let f = "state f(false)\n";
    let mixed = "if f then \"GET\" else \"AB\".bytes";
    let rows: &[support::Row] = &[
        (
            &["Clip as g | data=(if true then \"GET\" else \"AB\".bytes)\n"],
            &[],
        ),
        (
            &[
                f,
                "Clip as g | data=(if f then \"GET\" else \"AB\".bytes)\n",
            ],
            &[],
        ),
        (
            &[
                f,
                "Clip as g | data=(if f then \"AB\".bytes else \"GET\")\n",
            ],
            &[],
        ),
        (
            &[f, "Clip as g | data=(if f then \"GET\" else [71, 69])\n"],
            &[],
        ),
        (
            &[
                f,
                "fn m(x):\n    match x:\n        true: \"GET\"\n        else: \"AB\".bytes\n",
                "Clip as g | data=m(f)\n",
            ],
            &[],
        ),
        (
            &[
                f,
                "let d: u8[..3] = if f then \"GET\" else \"AB\".bytes\n",
                "Clip as g | data=d\n",
            ],
            &[],
        ),
        (
            &[
                f,
                "let d: u8[..2] = if f then \"GET\" else \"AB\".bytes\n",
                "Clip as g | data=d\n",
            ],
            &[(
                Code::CapacityExceeded,
                "let d: u8[..2] = if f then \"GET\" else \"AB\".bytes\n",
            )],
        ),
        (
            &[
                f,
                "Clip as g | fixed=(if f then \"GET\" else \"AB\".bytes)\n",
            ],
            &[(Code::VariableArray, mixed)],
        ),
        (
            &["Clip as g | fixed=(if true then \"GET\" else \"AB\".bytes)\n"],
            &[],
        ),
        (
            &[f, "Clip as g | data=(if f then \"GET\" else [1.5])\n"],
            &[(Code::TypeMismatch, "if f then \"GET\" else [1.5]")],
        ),
        (
            &[
                f,
                "let p = if f then \"GET\" else \"AB\".bytes\n",
                "Clip as g | data=p\n",
            ],
            &[(Code::TypeMismatch, mixed)],
        ),
        (
            &[
                f,
                "Clip as g | data=(if f then \"GET\".bytes else \"AB\".bytes)\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), rows);
    let rows: &[support::Row] = &[(
        &[
            f,
            "Song s:\n    Clip as c:\n        Note/(checksum(if f then \"a\" else \"bc\".bytes))\n",
        ],
        &[],
    )];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::native_vocabulary(), rows);
}

#[test]
fn a_let_whose_initializer_drew_an_error_is_a_placeholder_at_an_array_attribute() {
    let f = "state f(false)\n";
    let note = "state note(60)\n";
    let choice = "match f:\n    true: \"GET\"\n    else: \"AB\"\n";
    let reactive = format!("let m = {choice}");
    let annotated = format!("let m: u8[..8] = {choice}");
    let rows: &[support::Row] = &[
        (
            &[f, &reactive, "Clip as g | data=m\n"],
            &[(Code::ReactiveString, reactive.as_str())],
        ),
        (
            &[
                note,
                "let m = \"GET\" + label(note)\n",
                "Clip as g | data=m\n",
            ],
            &[(Code::CompileTimeNative, "label(note)")],
        ),
        (
            &[note, "let m = note + true\n", "Clip as g | data=m\n"],
            &[(Code::OperandType, "note + true")],
        ),
        (
            &[note, "let m = note + true\n", "Clip as g | fixed=m\n"],
            &[(Code::OperandType, "note + true")],
        ),
        (
            &[
                f,
                "let m = if f then \"G\" else \"A\"\n",
                "Clip as g:\n    Note | level=m\n",
            ],
            &[(Code::ReactiveString, "let m = if f then \"G\" else \"A\"\n")],
        ),
        (&[f, &annotated, "Clip as g | data=m\n"], &[]),
        (&["let m = \"GET\"\n", "Clip as g | data=m\n"], &[]),
        (
            &["Clip as g | fixed=((1 + true) + [1, 2][5])\n"],
            &[
                (Code::OperandType, "1 + true"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), rows);
}

#[test]
fn beside_a_placeholder_an_operand_draws_only_what_its_expression_draws_for_every_value() {
    let x = "state x(1)\n";
    let flag = "state flag(true)\n";
    let rows: &[support::Row] = &[
        (
            &[x, "Clip c:\n    Note/(x + 1.5 + 2.5 + 3.5 + 4.5)\n"],
            &[(Code::TypeMismatch, "x + 1.5 + 2.5 + 3.5 + 4.5")],
        ),
        (
            &["Clip c:\n    Note/(nothing + 1.5 + 2.5 + 3.5 + 4.5)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[x, "Clip c:\n    Note/(x + 300000000000 + 1.5)\n"],
            &[(Code::TypeMismatch, "x + 300000000000 + 1.5")],
        ),
        (
            &["Clip c:\n    Note/(nothing + 300000000000 + 1.5)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (&[x, "Clip c:\n    Note/([x, 1.5, 2.5][0])\n"], &[]),
        (
            &["Clip c:\n    Note/([nothing, 1.5, 2.5][0])\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (&[x, "Clip c:\n    Note/([x, 1.5, 2.5][^3])\n"], &[]),
        (
            &["Clip c:\n    Note/([nothing, 1.5, 2.5][^3])\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (&[x, "Clip c:\n    Note/([x, [1, 2][5]][0])\n"], &[]),
        (
            &["Clip c:\n    Note/([nothing, [1, 2][5]][0])\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[x, "Clip c:\n    Note/([x, 1.5][1])\n"],
            &[(Code::TypeMismatch, "[x, 1.5][1]")],
        ),
        (
            &["Clip c:\n    Note/([nothing, 1.5][1])\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[flag, "Clip c:\n    Note/(if flag then 1.5 else 2.5)\n"],
            &[(Code::TypeMismatch, "if flag then 1.5 else 2.5")],
        ),
        (
            &["Clip c:\n    Note/(if nothing then 1.5 else 2.5)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[x, "Clip c:\n    Note/(if true then x else 1.5)\n"],
            &[(Code::TypeMismatch, "if true then x else 1.5")],
        ),
        (
            &["Clip c:\n    Note/(if true then nothing else 1.5)\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::TypeMismatch, "if true then nothing else 1.5"),
            ],
        ),
        (
            &[flag, "Clip c:\n    Note when flag or 1 or 2\n"],
            &[(Code::TypeMismatch, "1"), (Code::TypeMismatch, "2")],
        ),
        (
            &["Clip c:\n    Note when nothing or 1 or 2\n"],
            &[(Code::UnknownName, "nothing"), (Code::TypeMismatch, "2")],
        ),
        (
            &["Clip c:\n    Note | length=(1bars + 2s + 3s)\n"],
            &[(Code::MixedFamilies, "1bars + 2s")],
        ),
        (
            &["Clip c:\n    Note | length=(nothing + 2s + 3s)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[x, "fn g(a): a + 1.5 + 2.5\n", "Clip c:\n    Note/(g(x))\n"],
            &[(Code::TypeMismatch, "a + 1.5 + 2.5")],
        ),
        (
            &[
                "fn g(a): a + 1.5 + 2.5\n",
                "Clip c:\n    Note/(g(nothing))\n",
            ],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[
                x,
                "Clip c:\n    Note/((if true then [x, 1.5] else [2.5, 3])[0])\n",
            ],
            &[(
                Code::TypeMismatch,
                "(if true then [x, 1.5] else [2.5, 3])[0]",
            )],
        ),
        (
            &["Clip c:\n    Note/((if true then [nothing, 1.5] else [2.5, 3])[0])\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[
                "state note(60)\n",
                "state s = note + 1.5 + 2.5\n",
                "Clip c:\n    Note/(s + note)\n",
            ],
            &[(Code::TypeMismatch, "s + note")],
        ),
        (
            &[
                "state note(60)\n",
                "state s = nothing + 1.5 + 2.5\n",
                "Clip c:\n    Note/(s + note)\n",
            ],
            &[(Code::UnknownName, "nothing")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
    let rows: &[support::Row] = &[
        (
            &["Env e | pitch=(2ms + 1ms)\n"],
            &[
                (Code::TypeMismatch, "2ms + 1ms"),
                (Code::UnitMismatch, "2ms + 1ms"),
            ],
        ),
        (
            &["Env e | pitch=(nothing + 2ms + 1ms)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::envelope_vocabulary(), rows);
    let rows: &[support::Row] = &[
        (
            &[
                "let nothing = \"AB\"\n",
                "Clip c | data=(nothing + \"GET\")\n",
            ],
            &[],
        ),
        (
            &["Clip c | data=(nothing + \"GET\")\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["Clip c | data=(if nothing then \"GET\" else \"AB\")\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["Clip c | fixed=(nothing + \"GET\")\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["Clip c:\n    Note | level=(\"A\" + \"GET\")\n"],
            &[(Code::TypeMismatch, "\"A\" + \"GET\"")],
        ),
        (
            &["Clip c:\n    Note | level=(nothing + \"GET\")\n"],
            &[(Code::UnknownName, "nothing")],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), rows);
}

#[test]
fn the_operand_that_keeps_a_flow_beside_a_placeholder_is_chosen_in_the_expressions_order() {
    let x = "state x(1)\n";
    let f = "fn f(v): v * 2\n";
    let g = "fn g(v): v\n";
    let rows: &[support::Row] = &[
        (
            &[f, "Clip c:\n    Note/(nothing + 1.5 + f(2.5))\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[x, f, "Clip c:\n    Note/(x + 1.5 + f(2.5))\n"],
            &[(Code::TypeMismatch, "x + 1.5 + f(2.5)")],
        ),
        (
            &[g, "Clip c:\n    Note/(nothing + g(2.5) + 1.5)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[x, g, "Clip c:\n    Note/(x + g(2.5) + 1.5)\n"],
            &[(Code::TypeMismatch, "x + g(2.5) + 1.5")],
        ),
        (
            &[f, "Clip c:\n    Note/(nothing + f(2.5))\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[f, "Clip c:\n    Note/(1 + f(2.5))\n"],
            &[(Code::TypeMismatch, "1 + f(2.5)")],
        ),
        (
            &[f, "Clip c:\n    Note/(nothing + f(2))\n"],
            &[(Code::UnknownName, "nothing")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn beside_a_placeholder_an_operand_draws_only_where_its_twin_draws_with_its_twins_code() {
    let x = "state x(1)\n";
    let rows: &[support::Row] = &[
        (
            &["Clip c:\n    Note/(nothing + 1.5 + (2 + true))\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::OperandType, "2 + true"),
            ],
        ),
        (
            &[x, "Clip c:\n    Note/(x + 1.5 + (2 + true))\n"],
            &[(Code::OperandType, "2 + true")],
        ),
        (
            &["Clip c:\n    Note/(nothing + 1.5 + i32(true))\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::ConversionType, "i32(true)"),
            ],
        ),
        (
            &["Clip c:\n    Note when nothing and 1\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &["Clip c:\n    Note/(if (1 + true) then nothing else 2.5)\n"],
            &[
                (Code::OperandType, "1 + true"),
                (Code::UnknownName, "nothing"),
            ],
        ),
        (
            &[x, "Clip c:\n    Note/(if (1 + true) then x else 2.5)\n"],
            &[(Code::OperandType, "1 + true")],
        ),
        (
            &["Clip c:\n    Note/((nothing - true) + true)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[x, "Clip c:\n    Note/((x - true) + true)\n"],
            &[(Code::OperandType, "x - true")],
        ),
        (
            &["Clip c:\n    Note when nothing < true\n"],
            &[(Code::UnknownName, "nothing")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
    let rows: &[support::Row] = &[
        (
            &["Env e | attack=(nothing * 2ms + true)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
        (
            &[x, "Env e | attack=(x * 2ms + true)\n"],
            &[(Code::OperandType, "x * 2ms + true")],
        ),
    ];
    let envelope = symphonyscript_fixtures::envelope_vocabulary();
    support::assert_rows_in_every_order(&envelope, rows);
    let rows: &[support::Row] = &[
        (
            &["Clip as g | three=([nothing, 1.5] + [(2 + true)])\n"],
            &[
                (Code::UnknownName, "nothing"),
                (Code::OperandType, "2 + true"),
            ],
        ),
        (
            &[x, "Clip as g | three=([x, 1.5] + [(2 + true)])\n"],
            &[(Code::OperandType, "2 + true")],
        ),
    ];
    support::assert_rows_in_every_order(&support::signature_vocabulary(), rows);
    let compiler = symphonyscript::Compiler::new(&envelope).expect("it compiles");
    let text = "state x(1)\nEnv e | attack=(x * 2ms + true)\n";
    let summaries: Vec<String> = support::errors(compiler.check(&[support::source(text)]))
        .into_iter()
        .filter(|diagnostic| diagnostic.code == Code::OperandType)
        .map(|diagnostic| diagnostic.summary)
        .collect();
    assert_eq!(
        summaries,
        ["`+` takes two numbers or two strings"],
        "{text}"
    );
}

#[test]
fn an_own_typed_fault_inside_a_position_draws_whether_or_not_the_position_draws_a_type_mismatch() {
    let x = "state x(1)\n";
    let rows: &[support::Row] = &[
        (
            &["Clip c:\n    Note/(1.5 + [1, 2][5])\n"],
            &[
                (Code::TypeMismatch, "1.5 + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &[x, "Clip c:\n    Note/(x + 1.5 + [1, 2][5])\n"],
            &[
                (Code::TypeMismatch, "x + 1.5 + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note/(if [1, 2][5] > 0 then 1.5 else 2.5)\n"],
            &[
                (Code::TypeMismatch, "if [1, 2][5] > 0 then 1.5 else 2.5"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note/(1.5 + 300000000000)\n"],
            &[
                (Code::TypeMismatch, "1.5 + 300000000000"),
                (Code::DoesNotFit, "300000000000"),
            ],
        ),
        (
            &["Clip c:\n    Note/(1.5 + i32(true))\n"],
            &[
                (Code::TypeMismatch, "1.5 + i32(true)"),
                (Code::ConversionType, "i32(true)"),
            ],
        ),
        (
            &["Synth s:\n    Osc | freq=f64(true)\n"],
            &[(Code::ConversionType, "f64(true)")],
        ),
        (
            &["Clip c:\n    Note/(i32(true) + 1)\n"],
            &[(Code::ConversionType, "i32(true)")],
        ),
        (
            &["state flag(true)\n", "Synth s:\n    Osc | freq=f64(flag)\n"],
            &[(Code::ConversionType, "f64(flag)")],
        ),
        (
            &["Clip c:\n    Note/([1, 2][i32(true)])\n"],
            &[(Code::ConversionType, "i32(true)")],
        ),
        (
            &[x, "Clip c:\n    Note/(x + 1.5 + i32(true))\n"],
            &[
                (Code::TypeMismatch, "x + 1.5 + i32(true)"),
                (Code::ConversionType, "i32(true)"),
            ],
        ),
        (
            &["Clip c:\n    Note/(1.5 + [1, 2][1])\n"],
            &[(Code::TypeMismatch, "1.5 + [1, 2][1]")],
        ),
        (
            &["Clip c:\n    Note/([1, 2][5] + 1)\n"],
            &[(Code::IndexOutOfRange, "5")],
        ),
        (&["Clip c:\n    Note/(60 + [1, 2][1])\n"], &[]),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

const LEAVES: [&str; 10] = [
    "q",
    "1",
    "1.5",
    "true",
    "Wave::Saw",
    "\"a\"",
    "[1, 2]",
    "[1, 2][5]",
    "300000000000",
    "i32(true)",
];
const NAME: usize = 0;
const OUT_OF_RANGE: usize = 7;
const TOO_LARGE: usize = 8;
const CONVERTED_BOOL: usize = 9;
const BINARY: [&str; 10] = ["+", "-", "*", "/", "div", "%", "and", "or", "<", "is"];
const UNARY: [&str; 6] = ["-", "not", "i32", "u8", "len", "bytes"];

#[derive(Clone)]
enum Generated {
    Leaf(usize),
    Unary(usize, Box<Generated>),
    Binary(usize, Box<Generated>, Box<Generated>),
    If(Box<Generated>, Box<Generated>, Box<Generated>),
    Index(Box<Generated>, Box<Generated>),
    Slice(Box<Generated>, Box<Generated>),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Reach {
    Read,
    Unread,
    Unsure,
}

#[derive(Clone, PartialEq, Debug)]
enum Model {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str,
    Wave,
    Array(Vec<i128>),
    Fault,
    Placeholder,
    Unsure,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Int,
    Float,
    Bool,
    Str,
    Wave,
    Array,
    Fault,
    Placeholder,
    Unsure,
}

struct LeafSite {
    leaf: usize,
    start: usize,
    end: usize,
    read: Reach,
    takes_type: bool,
}

impl Generated {
    fn holds_name(&self) -> bool {
        match self {
            Generated::Leaf(leaf) => *leaf == NAME,
            Generated::Unary(_, operand) => operand.holds_name(),
            Generated::Binary(_, lhs, rhs)
            | Generated::Index(lhs, rhs)
            | Generated::Slice(lhs, rhs) => lhs.holds_name() || rhs.holds_name(),
            Generated::If(condition, then, otherwise) => {
                condition.holds_name() || then.holds_name() || otherwise.holds_name()
            }
        }
    }

    fn compound(&self) -> bool {
        !matches!(self, Generated::Leaf(_))
    }

    fn form(&self) -> usize {
        match self {
            Generated::Leaf(leaf) => *leaf,
            Generated::Unary(op, _) => LEAVES.len() + op,
            Generated::Binary(op, ..) => LEAVES.len() + UNARY.len() + op,
            Generated::Index(..) => LEAVES.len() + UNARY.len() + BINARY.len(),
            Generated::Slice(..) => LEAVES.len() + UNARY.len() + BINARY.len() + 1,
            Generated::If(..) => LEAVES.len() + UNARY.len() + BINARY.len() + 2,
        }
    }

    fn class(&self) -> usize {
        match self {
            Generated::Leaf(leaf) => *leaf,
            Generated::Unary(..) => LEAVES.len(),
            Generated::Binary(..) => LEAVES.len() + 1,
            Generated::Index(..) => LEAVES.len() + 2,
            Generated::Slice(..) => LEAVES.len() + 3,
            Generated::If(..) => LEAVES.len() + 4,
        }
    }

    fn operand_classes(&self) -> [usize; 3] {
        let absent = LEAVES.len() + 5;
        match self {
            Generated::Leaf(_) => [absent; 3],
            Generated::Unary(_, operand) => [operand.class(), absent, absent],
            Generated::Binary(_, lhs, rhs)
            | Generated::Index(lhs, rhs)
            | Generated::Slice(lhs, rhs) => [lhs.class(), rhs.class(), absent],
            Generated::If(condition, then, otherwise) => {
                [condition.class(), then.class(), otherwise.class()]
            }
        }
    }

    fn print(&self, out: &mut String, read: Reach, sites: &mut Vec<LeafSite>) {
        match self {
            Generated::Leaf(leaf) => {
                let start = out.len();
                out.push_str(LEAVES[*leaf]);
                sites.push(LeafSite {
                    leaf: *leaf,
                    start,
                    end: out.len(),
                    read,
                    takes_type: false,
                });
            }
            Generated::Unary(op, operand) => {
                match UNARY[*op] {
                    "-" => out.push('-'),
                    "not" => out.push_str("not "),
                    member @ ("len" | "bytes") => {
                        out.push('(');
                        operand.print(out, read, sites);
                        out.push_str(if member == "len" { ").size" } else { ").bytes" });
                        return;
                    }
                    name => {
                        out.push_str(name);
                        out.push('(');
                        operand.print(out, read, sites);
                        out.push(')');
                        return;
                    }
                }
                operand.operand(out, read, sites);
            }
            Generated::Binary(op, lhs, rhs) => {
                let first = sites.len();
                lhs.operand(out, read, sites);
                out.push(' ');
                out.push_str(BINARY[*op]);
                out.push(' ');
                let second = sites.len();
                rhs.operand(out, read, sites);
                if ["+", "-", "*", "%", "<", "is"].contains(&BINARY[*op]) {
                    for (own, partner, at) in [(lhs, rhs, first), (rhs, lhs, second)] {
                        if matches!(**own, Generated::Leaf(TOO_LARGE))
                            && partner.fixed_integer()
                            && let Some(site) = sites.get_mut(at)
                        {
                            site.takes_type = true;
                        }
                    }
                }
            }
            Generated::If(condition, then, otherwise) => {
                out.push_str("if ");
                condition.operand(out, read, sites);
                let (then_read, otherwise_read) = match read {
                    Reach::Read => match condition.value() {
                        Model::Bool(true) => (Reach::Read, Reach::Unread),
                        Model::Bool(false) => (Reach::Unread, Reach::Read),
                        Model::Unsure => (Reach::Unsure, Reach::Unsure),
                        _ => (Reach::Unread, Reach::Unread),
                    },
                    other => (other, other),
                };
                out.push_str(" then ");
                then.operand(out, then_read, sites);
                out.push_str(" else ");
                otherwise.operand(out, otherwise_read, sites);
            }
            Generated::Index(base, index) => {
                base.operand(out, read, sites);
                out.push('[');
                index.print(out, read, sites);
                out.push(']');
            }
            Generated::Slice(base, bound) => {
                base.operand(out, read, sites);
                out.push('[');
                bound.print(out, read, sites);
                out.push_str("..]");
            }
        }
    }

    fn operand(&self, out: &mut String, read: Reach, sites: &mut Vec<LeafSite>) {
        if self.compound() {
            out.push('(');
            self.print(out, read, sites);
            out.push(')');
        } else {
            self.print(out, read, sites);
        }
    }

    fn fixed_integer(&self) -> bool {
        match self {
            Generated::Unary(op, operand) => {
                matches!(UNARY[*op], "i32" | "u8")
                    && !matches!(
                        operand.kind(),
                        Kind::Array | Kind::Placeholder | Kind::Unsure
                    )
            }
            Generated::Leaf(leaf) => *leaf == CONVERTED_BOOL,
            Generated::Binary(..)
            | Generated::If(..)
            | Generated::Index(..)
            | Generated::Slice(..) => false,
        }
    }

    fn kind(&self) -> Kind {
        match self {
            Generated::Leaf(leaf) => match LEAVES[*leaf] {
                "q" => Kind::Unsure,
                "1.5" => Kind::Float,
                "true" => Kind::Bool,
                "Wave::Saw" => Kind::Wave,
                "\"a\"" => Kind::Str,
                "[1, 2]" => Kind::Array,
                _ => Kind::Int,
            },
            Generated::Unary(op, operand) => {
                let inner = operand.kind();
                match (UNARY[*op], inner) {
                    (_, Kind::Placeholder) => Kind::Placeholder,
                    (_, Kind::Unsure) | ("-" | "i32" | "u8", Kind::Array) => Kind::Unsure,
                    ("not", _) => Kind::Bool,
                    ("i32" | "u8", _) => Kind::Int,
                    ("-", Kind::Int | Kind::Float) => inner,
                    ("len", Kind::Array) => Kind::Int,
                    ("bytes", Kind::Array) => Kind::Array,
                    ("len" | "bytes", Kind::Str) => Kind::Unsure,
                    ("len", _) => Kind::Placeholder,
                    _ => Kind::Fault,
                }
            }
            Generated::Binary(op, lhs, rhs) => {
                let (a, b) = (lhs.kind(), rhs.kind());
                let numeric = |kind: Kind| matches!(kind, Kind::Int | Kind::Float);
                match (BINARY[*op], a, b) {
                    (_, Kind::Placeholder, _) | (_, _, Kind::Placeholder) => Kind::Placeholder,
                    (_, Kind::Unsure, _) | (_, _, Kind::Unsure) => Kind::Unsure,
                    ("+", Kind::Array, Kind::Array) => Kind::Array,
                    (_, Kind::Array, _) | (_, _, Kind::Array) => Kind::Placeholder,
                    ("and" | "or" | "<" | "is", ..) => Kind::Bool,
                    ("+", Kind::Str, Kind::Str) => Kind::Str,
                    ("/", a, b) if numeric(a) && numeric(b) => Kind::Float,
                    ("div", Kind::Int, Kind::Int) => Kind::Int,
                    ("div", ..) => Kind::Fault,
                    (_, Kind::Int, Kind::Int) => Kind::Int,
                    (_, a, b) if numeric(a) && numeric(b) => Kind::Float,
                    _ => Kind::Fault,
                }
            }
            Generated::If(condition, then, otherwise) => {
                let (c, a, b) = (condition.kind(), then.kind(), otherwise.kind());
                if [c, a, b].contains(&Kind::Placeholder) {
                    return Kind::Placeholder;
                }
                if [c, a, b].contains(&Kind::Unsure) {
                    return Kind::Unsure;
                }
                if c == Kind::Array {
                    return Kind::Placeholder;
                }
                if c != Kind::Bool || a == Kind::Fault || b == Kind::Fault {
                    return Kind::Fault;
                }
                match (a, b) {
                    (a, b) if a == b => a,
                    (Kind::Int, Kind::Float) | (Kind::Float, Kind::Int) => Kind::Float,
                    (Kind::Array, _) | (_, Kind::Array) => Kind::Unsure,
                    _ => Kind::Fault,
                }
            }
            Generated::Index(base, index) => match (base.kind(), index.kind()) {
                (Kind::Placeholder, _) | (_, Kind::Placeholder) => Kind::Placeholder,
                (Kind::Unsure | Kind::Str, _) | (_, Kind::Unsure) => Kind::Unsure,
                (Kind::Array, Kind::Int | Kind::Fault) => Kind::Int,
                (Kind::Array, Kind::Array) => Kind::Unsure,
                _ => Kind::Placeholder,
            },
            Generated::Slice(base, bound) => match (base.kind(), bound.kind()) {
                (Kind::Placeholder, _) | (_, Kind::Placeholder) => Kind::Placeholder,
                (Kind::Unsure | Kind::Str, _) | (_, Kind::Unsure) => Kind::Unsure,
                (Kind::Array, Kind::Int) => Kind::Array,
                (Kind::Array, _) => Kind::Unsure,
                _ => Kind::Placeholder,
            },
        }
    }

    fn value(&self) -> Model {
        match self.kind() {
            Kind::Unsure => return Model::Unsure,
            Kind::Placeholder => return Model::Placeholder,
            Kind::Fault => return Model::Fault,
            Kind::Int | Kind::Float | Kind::Bool | Kind::Str | Kind::Wave | Kind::Array => {}
        }
        let small = |value: &Model| match value {
            Model::Int(value) => value.abs() < 1 << 24,
            _ => true,
        };
        match self {
            Generated::Leaf(leaf) => match LEAVES[*leaf] {
                "1" => Model::Int(1),
                "300000000000" => Model::Int(300_000_000_000),
                "1.5" => Model::Float(1.5),
                "true" => Model::Bool(true),
                "Wave::Saw" => Model::Wave,
                "\"a\"" => Model::Str,
                "[1, 2]" => Model::Array(vec![1, 2]),
                _ => Model::Fault,
            },
            Generated::Unary(op, operand) => match (UNARY[*op], operand.value()) {
                (_, Model::Unsure) => Model::Unsure,
                (_, Model::Fault | Model::Placeholder) => Model::Fault,
                ("-", Model::Int(value)) => Model::Int(-value),
                ("-", Model::Float(value)) => Model::Float(-value),
                ("not", Model::Bool(value)) => Model::Bool(!value),
                ("len", Model::Array(items)) => Model::Int(items.len() as i128),
                ("not", _) => Model::Fault,
                _ => Model::Unsure,
            },
            Generated::Binary(op, lhs, rhs) => {
                let (a, b) = (lhs.value(), rhs.value());
                if a == Model::Unsure || b == Model::Unsure {
                    return Model::Unsure;
                }
                if matches!(a, Model::Fault | Model::Placeholder)
                    || matches!(b, Model::Fault | Model::Placeholder)
                {
                    return Model::Fault;
                }
                let mixed = matches!(
                    (&a, &b),
                    (Model::Int(_), Model::Float(_)) | (Model::Float(_), Model::Int(_))
                );
                if mixed && !(small(&a) && small(&b)) {
                    return Model::Unsure;
                }
                let float = |value: &Model| match value {
                    Model::Int(value) => Some(*value as f64),
                    Model::Float(value) => Some(*value),
                    _ => None,
                };
                match (BINARY[*op], &a, &b) {
                    ("and", Model::Bool(x), Model::Bool(y)) => Model::Bool(*x && *y),
                    ("or", Model::Bool(x), Model::Bool(y)) => Model::Bool(*x || *y),
                    ("is", Model::Bool(x), Model::Bool(y)) => Model::Bool(x == y),
                    ("is", Model::Wave, Model::Wave) => Model::Bool(true),
                    ("is", Model::Str, Model::Str) => Model::Bool(true),
                    ("is", Model::Int(x), Model::Int(y)) => Model::Bool(x == y),
                    ("<", Model::Int(x), Model::Int(y)) => Model::Bool(x < y),
                    ("is" | "<", x, y) => match (float(x), float(y)) {
                        (Some(x), Some(y)) if BINARY[*op] == "is" => Model::Bool(x == y),
                        (Some(x), Some(y)) => Model::Bool(x < y),
                        _ => Model::Fault,
                    },
                    ("+", Model::Array(x), Model::Array(y)) => {
                        Model::Array(x.iter().chain(y).copied().collect())
                    }
                    ("+", Model::Int(x), Model::Int(y)) => Model::Int(x + y),
                    ("-", Model::Int(x), Model::Int(y)) => Model::Int(x - y),
                    ("*", Model::Int(x), Model::Int(y)) => Model::Int(x * y),
                    ("div", Model::Int(x), Model::Int(y)) if *y != 0 => {
                        Model::Int(x.div_euclid(*y) - i128::from(y < &0 && x.rem_euclid(*y) != 0))
                    }
                    ("%", Model::Int(x), Model::Int(y)) if *y != 0 => {
                        let remainder = x % y;
                        Model::Int(if remainder != 0 && (remainder < 0) != (*y < 0) {
                            remainder + y
                        } else {
                            remainder
                        })
                    }
                    ("+" | "-" | "*" | "/", x, y) => match (float(x), float(y)) {
                        (Some(x), Some(y)) => Model::Float(match BINARY[*op] {
                            "+" => x + y,
                            "-" => x - y,
                            "*" => x * y,
                            _ => x / y,
                        }),
                        _ => Model::Fault,
                    },
                    _ => Model::Unsure,
                }
            }
            Generated::If(condition, then, otherwise) => match condition.value() {
                Model::Bool(true) => then.value(),
                Model::Bool(false) => otherwise.value(),
                Model::Unsure => Model::Unsure,
                _ => Model::Fault,
            },
            Generated::Index(base, index) => match (base.value(), index.value()) {
                (Model::Unsure, _) | (_, Model::Unsure) => Model::Unsure,
                (Model::Array(items), Model::Int(position)) => usize::try_from(position)
                    .ok()
                    .and_then(|position| items.get(position).copied())
                    .map_or(Model::Fault, Model::Int),
                _ => Model::Fault,
            },
            Generated::Slice(base, bound) => match (base.value(), bound.value()) {
                (Model::Unsure, _) | (_, Model::Unsure) => Model::Unsure,
                (Model::Array(items), Model::Int(from)) => {
                    let from = usize::try_from(from.clamp(0, items.len() as i128)).unwrap_or(0);
                    Model::Array(items.get(from..).unwrap_or_default().to_vec())
                }
                _ => Model::Fault,
            },
        }
    }
}

fn generated(depth: usize) -> Vec<Generated> {
    let leaves: Vec<Generated> = (0..LEAVES.len()).map(Generated::Leaf).collect();
    if depth <= 1 {
        return leaves;
    }
    let below = generated(depth - 1);
    let mut found = leaves;
    for operand in &below {
        for op in 0..UNARY.len() {
            found.push(Generated::Unary(op, Box::new(operand.clone())));
        }
    }
    for lhs in &below {
        for rhs in &below {
            for op in 0..BINARY.len() {
                found.push(Generated::Binary(
                    op,
                    Box::new(lhs.clone()),
                    Box::new(rhs.clone()),
                ));
            }
            found.push(Generated::Index(
                Box::new(lhs.clone()),
                Box::new(rhs.clone()),
            ));
            found.push(Generated::Slice(
                Box::new(lhs.clone()),
                Box::new(rhs.clone()),
            ));
        }
    }
    for condition in &below {
        for then in &below {
            for otherwise in &below {
                found.push(Generated::If(
                    Box::new(condition.clone()),
                    Box::new(then.clone()),
                    Box::new(otherwise.clone()),
                ));
            }
        }
    }
    found
}

fn sampled_depth_three(pool: &[Generated], count: usize) -> Vec<Generated> {
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % bound as u64) as usize
    };
    let compound: Vec<&Generated> = pool.iter().filter(|expr| expr.compound()).collect();
    let pick = |next: &mut dyn FnMut(usize) -> usize| -> Generated {
        if next(3) == 0 {
            pool[next(pool.len())].clone()
        } else {
            compound[next(compound.len())].clone()
        }
    };
    (0..count)
        .map(|_| {
            let shape = next(5);
            let first = pick(&mut next);
            let second = pick(&mut next);
            match shape {
                0 => Generated::Unary(next(UNARY.len()), Box::new(first)),
                1 => Generated::Binary(next(BINARY.len()), Box::new(first), Box::new(second)),
                2 => Generated::If(Box::new(first), Box::new(second), Box::new(pick(&mut next))),
                3 => Generated::Index(Box::new(first), Box::new(second)),
                _ => Generated::Slice(Box::new(first), Box::new(second)),
            }
        })
        .collect()
}

struct Placed {
    vocabulary: usize,
    items: Vec<String>,
    offset: usize,
    takes_types: bool,
}

fn placed(expression: &str) -> Vec<Placed> {
    let within = |head: &str, tail: &str, vocabulary: usize, takes_types: bool| Placed {
        vocabulary,
        items: vec![format!("{head}{expression}{tail}")],
        offset: head.len(),
        takes_types,
    };
    let annotated = "let a: i32 = ";
    vec![
        within("Clip c:\n    Note/(", ")\n", 0, true),
        within("Clip c:\n    Note when (", ")\n", 0, false),
        within("Clip c:\n    Note | velocity=(", ")\n", 0, true),
        within("Clip c | data=(", ")\n", 1, false),
        Placed {
            vocabulary: 0,
            items: vec![
                format!("{annotated}{expression}\n"),
                "Clip c:\n    Note/(a)\n".to_string(),
            ],
            offset: annotated.len(),
            takes_types: true,
        },
    ]
}

type Spans = std::collections::BTreeSet<(Code, String, u32, u32)>;

fn spanned_errors(compiler: &Compiler, sources: &[Source]) -> Spans {
    support::errors(compiler.check(sources))
        .into_iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                diagnostic.span.file.clone(),
                diagnostic.span.range.start,
                diagnostic.span.range.end,
            )
        })
        .collect()
}

fn common_to_twins(compiler: &Compiler, program: &str) -> Spans {
    let twins = [
        "state q(0): i32\n",
        "let q = 1\n",
        "state q(0): u8\n",
        "let q = u8(1)\n",
        "state q(0): i64\n",
        "let q = i64(1)\n",
        "state q(0.5): f32\n",
        "let q = f32(0.5)\n",
        "state q(0.5): f64\n",
        "let q = 0.5\n",
        "state q(false)\n",
        "let q = false\n",
        "state q(Wave::Sine)\n",
        "let q = Wave::Sine\n",
        "let q = \"a\"\n",
        "state q([]): i32[..4]\n",
        "let q = [1]\n",
        "let q = c\n",
    ];
    let mut common: Option<Spans> = None;
    for twin in twins {
        let found: Spans = spanned_errors(
            compiler,
            &[
                Source {
                    path: "main.sym".into(),
                    text: program.to_string(),
                },
                Source {
                    path: "twin.sym".into(),
                    text: twin.to_string(),
                },
            ],
        )
        .into_iter()
        .filter(|(_, file, ..)| file == "main.sym")
        .collect();
        common = Some(match common {
            None => found,
            Some(common) => common.intersection(&found).cloned().collect(),
        });
    }
    common.unwrap_or_default()
}

fn own_fault(site: &LeafSite, base: u32, whole: bool) -> Option<((Code, String, u32, u32), bool)> {
    let (code, start, end) = match site.leaf {
        OUT_OF_RANGE => (Code::IndexOutOfRange, site.end - 2, site.end - 1),
        TOO_LARGE => (Code::DoesNotFit, site.start, site.end),
        CONVERTED_BOOL => (Code::ConversionType, site.start, site.end),
        _ => return None,
    };
    let required = match (site.read, site.leaf) {
        (Reach::Read, TOO_LARGE) => site.takes_type || whole,
        (_, CONVERTED_BOOL) | (Reach::Read, _) => true,
        (Reach::Unread | Reach::Unsure, _) => false,
    };
    let key = (
        code,
        "main.sym".to_string(),
        base + start as u32,
        base + end as u32,
    );
    Some((key, required))
}

#[test]
fn every_fault_beside_another_fault_draws_by_the_placeholder_rule() {
    let (failures, programs) = covered(|rows| placeholder_rule_failures(rows, 0, 1));
    assert_placeholder_rule(&failures, programs, 457);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_every_fault_beside_another_fault_draws_by_the_placeholder_rule_in_the_odd_expressions()
 {
    let (failures, programs) = placeholder_rule_failures(&Rows::Every, 0, 2);
    assert_placeholder_rule(&failures, programs, 15_810);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_every_fault_beside_another_fault_draws_by_the_placeholder_rule_in_the_even_expressions()
 {
    let (failures, programs) = placeholder_rule_failures(&Rows::Every, 1, 2);
    assert_placeholder_rule(&failures, programs, 15_810);
}

fn placeholder_rule_failures(rows: &Rows, part: usize, parts: usize) -> (Vec<String>, usize) {
    let pool = generated(2);
    let mut expressions: Vec<Generated> = pool.clone();
    expressions.extend(sampled_depth_three(&pool, 3000));
    let compilers = [
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles"),
        Compiler::new(&support::signature_vocabulary()).expect("the vocabulary compiles"),
    ];
    let mut failures = Vec::new();
    let mut programs = 0;
    for expression in expressions.iter().skip(part).step_by(parts) {
        let mut text = String::new();
        let mut sites = Vec::new();
        expression.print(&mut text, Reach::Read, &mut sites);
        let [first, second, third] = expression.operand_classes();
        for (placement, place) in placed(&text).into_iter().enumerate() {
            let compiler = &compilers[place.vocabulary];
            for (direction, order) in
                support::permutations(&(0..place.items.len()).collect::<Vec<_>>())
                    .into_iter()
                    .enumerate()
            {
                let tag = [
                    expression.form(),
                    first,
                    second,
                    third,
                    placement,
                    direction,
                ];
                if !rows.take(&tag) {
                    continue;
                }
                let before: usize = order
                    .iter()
                    .take_while(|index| **index != 0)
                    .map(|index| place.items[*index].len())
                    .sum();
                let program: String = order
                    .iter()
                    .map(|index| place.items[*index].as_str())
                    .collect();
                let base = (before + place.offset) as u32;
                let found = spanned_errors(
                    compiler,
                    &[Source {
                        path: "main.sym".into(),
                        text: program.clone(),
                    }],
                );
                programs += 1;
                if expression.holds_name() {
                    let mut expected = common_to_twins(compiler, &program);
                    for site in sites.iter().filter(|site| site.leaf == NAME) {
                        expected.insert((
                            Code::UnknownName,
                            "main.sym".into(),
                            base + site.start as u32,
                            base + site.end as u32,
                        ));
                    }
                    if found != expected {
                        failures.push(format!(
                            "{program}\n  found {found:?}\n  twins {expected:?}"
                        ));
                    }
                    continue;
                }
                let whole = |site: &LeafSite| {
                    site.start == 0 && site.end == text.len() && place.takes_types
                };
                for site in &sites {
                    let Some((key, required)) = own_fault(site, base, whole(site)) else {
                        continue;
                    };
                    if required && !found.contains(&key) {
                        failures.push(format!("{program}\n  misses {key:?}\n  found {found:?}"));
                    }
                    let unread = site.read == Reach::Unread && site.leaf != CONVERTED_BOOL;
                    if unread && found.contains(&key) {
                        failures.push(format!("{program}\n  draws {key:?} in an unread part"));
                    }
                }
            }
        }
    }
    (failures, programs)
}

fn assert_placeholder_rule(failures: &[String], programs: usize, expected_programs: usize) {
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(programs, expected_programs);
}

#[test]
fn every_seed_row_of_the_placeholder_rule_draws_what_its_rule_gives() {
    let verse = "Clip verse:\n    Note/60\n";
    let rows: &[support::Row] = &[
        (
            &["Clip intro:\n    Plays: verse+\n", verse],
            &[(Code::ExpressionSyntax, "\n")],
        ),
        (
            &["state fill_on(false+)\n"],
            &[(Code::ExpressionSyntax, ")")],
        ),
        (&["let b = abs-(\"GET\")\n"], &[(Code::NotAValue, "abs")]),
        (
            &["Clip c:\n    Note/([1, 2, 3][1.5 + [1, 2][5]])\n"],
            &[
                (Code::IndexType, "1.5 + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    for i in 0..(1.5 + [1, 2][5]):\n        Note/60\n"],
            &[
                (Code::RangeBound, "1.5 + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note | velocity=rand(1.5 + [1, 2][5], 1)\n"],
            &[
                (Code::OperandType, "rand(1.5 + [1, 2][5], 1)"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note/(true + [1, 2][5])\n"],
            &[
                (Code::OperandType, "true + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note/(if 1 + [1, 2][5] then 1 else 2)\n"],
            &[
                (Code::ConditionType, "1 + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["let a: i32 = 1.5 + [1, 2][5]\n", "Clip c:\n    Note/(a)\n"],
            &[
                (Code::TypeMismatch, "1.5 + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note/(if [1, 2][5] > 0 then Wave::Saw else Wave::Sine)\n"],
            &[
                (
                    Code::TypeMismatch,
                    "if [1, 2][5] > 0 then Wave::Saw else Wave::Sine",
                ),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["Clip c:\n    Note/([1.5, 2.5][i32(true)])\n"],
            &[
                (Code::ConversionType, "i32(true)"),
                (Code::TypeMismatch, "[1.5, 2.5][i32(true)]"),
            ],
        ),
        (
            &["Clip c:\n    Note/(1.5 + [1, 2][i32(true)])\n"],
            &[
                (Code::ConversionType, "i32(true)"),
                (Code::TypeMismatch, "1.5 + [1, 2][i32(true)]"),
            ],
        ),
        (
            &["Clip c:\n    Note when not (1 + q)\n"],
            &[(Code::UnknownName, "q")],
        ),
        (
            &["Clip c:\n    Note when 1 + q\n"],
            &[(Code::UnknownName, "q")],
        ),
        (
            &["Clip c:\n    Note/((true and q) + 1)\n"],
            &[
                (Code::UnknownName, "q"),
                (Code::OperandType, "(true and q) + 1"),
            ],
        ),
        (
            &["Clip c:\n    Note/((q > 0) * 2)\n"],
            &[(Code::UnknownName, "q"), (Code::OperandType, "(q > 0) * 2")],
        ),
        (
            &["state b(true)\n", "Clip c:\n    Note/(b + [1, 2][5])\n"],
            &[
                (Code::OperandType, "b + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["fn h(v): v + 0.5\n", "Clip c:\n    Note/(h(q))\n"],
            &[(Code::UnknownName, "q")],
        ),
        (
            &["Clip c:\n    Note/([q, 1.5, 2][2])\n"],
            &[(Code::UnknownName, "q")],
        ),
        (
            &[
                "state section(0)\n",
                "let sections = [verse, chorus]\n",
                "Clip intro:\n    for s in sections:\n        Starts: s\n",
                "Clip outro:\n    Plays: sections[section]\n",
            ],
            &[(Code::UnknownName, "verse"), (Code::UnknownName, "chorus")],
        ),
        (
            &[
                "state gate(false)\n",
                "state rising = gate and not gate.past(q)\n",
                "Clip c:\n    Note when rising\n",
            ],
            &[(Code::UnknownName, "q")],
        ),
        (
            &["Clip c:\n    Note/(q + 1)\n"],
            &[(Code::UnknownName, "q")],
        ),
        (
            &["Clip c:\n    Note/([q, 1, 2][1])\n"],
            &[(Code::UnknownName, "q")],
        ),
        (
            &["Clip c:\n    Note/(1.5 + [1, 2][1])\n"],
            &[(Code::TypeMismatch, "1.5 + [1, 2][1]")],
        ),
        (
            &["Clip c:\n    Note/((if false then [1, 2][5] else 3) + 60)\n"],
            &[],
        ),
        (&["Clip intro:\n    Plays: verse\n", verse], &[]),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

const NON_NAME_PLACEHOLDERS: [(&str, Code, &str); 7] = [
    ("(1 +)", Code::ExpressionSyntax, ")"),
    ("(true +)", Code::ExpressionSyntax, ")"),
    ("(1.5 +)", Code::ExpressionSyntax, ")"),
    ("5[0]", Code::NotAnArray, "5"),
    ("true[0]", Code::NotAnArray, "true"),
    ("5.size", Code::UnknownAttribute, "5.size"),
    ("5[1..][0]", Code::NotAnArray, "5"),
];

const PLACEHOLDER_PARTNERS: [&str; 5] = ["true", "1.5", "300000000000", "\"a\"", "Wave::Saw"];

const PARTNER_POSITIONS: [&str; 4] = [
    "Clip c:\n    Note/({x})\n",
    "Clip c:\n    Note | velocity=({x})\n",
    "Clip c:\n    Note when {x}\n",
    "Clip c:\n    Note | length=({x})\n",
];

fn partner_rows(rows: &Rows) -> Vec<String> {
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (fault, (placeholder, code, spanned)) in NON_NAME_PLACEHOLDERS.iter().enumerate() {
        for (partner, value) in PLACEHOLDER_PARTNERS.iter().enumerate() {
            for (side, operator) in ["+", "*"].into_iter().enumerate() {
                for left in [false, true] {
                    for (position, place) in PARTNER_POSITIONS.iter().enumerate() {
                        if !rows.take(&[fault, partner, side, usize::from(left), position]) {
                            continue;
                        }
                        let expression = if left {
                            format!("{value} {operator} {placeholder}")
                        } else {
                            format!("{placeholder} {operator} {value}")
                        };
                        let text = place.replace("{x}", &expression);
                        let found = support::error_texts(&compiler, &text);
                        let expected = vec![(*code, (*spanned).to_string())];
                        if found != expected {
                            failures.push(format!(
                                "287: {text}\n  draws {found:?}\n  expected {expected:?}"
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
fn beside_a_placeholder_that_is_not_a_names_the_partner_is_checked_in_its_own_type() {
    let failures = covered(partner_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_beside_a_placeholder_that_is_not_a_names_the_partner_is_checked_in_its_own_type() {
    let failures = partner_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn beside_a_placeholder_that_is_not_a_names_the_valid_rows_hold() {
    let rows: &[support::Row] = &[
        (
            &["Clip c:\n    Note/((if false then [1, 2][5] else 3) + 60)\n"],
            &[],
        ),
        (
            &["Clip c:\n    Note/(1.5 + [1, 2][1])\n"],
            &[(Code::TypeMismatch, "1.5 + [1, 2][1]")],
        ),
        (
            &["Clip c:\n    Note/(nothing + true)\n"],
            &[(Code::UnknownName, "nothing")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

const FLOAT_FLOWS: [&str; 10] = [
    "u8(2) / 300",
    "300 / u8(2)",
    "i32(5) / 3000000000",
    "\"a\".bytes[0] / 256",
    "u8(250) / 400 + 0.5",
    "lerp(u8(2), 300, 0.5)",
    "u8(2) / 300 * 2",
    "sin(u8(2) * 300)",
    "2 ** (u8(7) / 1200)",
    "floor(u8(2) / 300)",
];

const FLOAT_STANDS: [&str; 3] = [
    "Clip c:\n    Note | velocity=({x})\n",
    "Clip c:\n    Note | length=({x})\n",
    "Clip c | gain=({x}):\n    Note\n",
];

const CALL_RESULTS: [[&str; 2]; 3] = [
    [
        "fn r(n): if n > 0 then r(n - 1) else u8(250) + 300\n",
        "Clip c:\n    Note/(r({n}))\n",
    ],
    [
        "fn r(a, n): if n > 0 then r(a, n - 1) else a + 300\n",
        "Clip c:\n    Note/(r(u8(250), {n}))\n",
    ],
    [
        "fn r(a, n): if n > 0 then r(a, n - 1) else a - u8(5)\n",
        "Clip c:\n    Note/(r(u8(3), {n}))\n",
    ],
];

fn flowing_float_rows(rows: &Rows) -> Vec<String> {
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (form, expression) in FLOAT_FLOWS.iter().enumerate() {
        for (stand, place) in FLOAT_STANDS.iter().enumerate() {
            if !rows.take(&[0, form, stand, 0]) {
                continue;
            }
            let text = place.replace("{x}", expression);
            let found = support::error_texts(&compiler, &text);
            if !found.is_empty() {
                failures.push(format!("292: {text}\n  draws {found:?}"));
            }
        }
    }
    for (form, [function, call]) in CALL_RESULTS.iter().enumerate() {
        for (depth, n) in ["0", "1", "3"].into_iter().enumerate() {
            for reversed in [false, true] {
                if !rows.take(&[1, form, depth, usize::from(reversed)]) {
                    continue;
                }
                let read = call.replace("{n}", n);
                let text = if reversed {
                    format!("{read}{function}")
                } else {
                    format!("{function}{read}")
                };
                let found = support::error_texts(&compiler, &text);
                if !found.is_empty() {
                    failures.push(format!("292: {text}\n  draws {found:?}"));
                }
            }
        }
    }
    failures
}

#[test]
fn a_literal_beside_a_fixed_type_operand_takes_the_flowing_float_type_and_a_recursive_result_the_calls_type()
 {
    let failures = covered(flowing_float_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_literal_beside_a_fixed_type_operand_takes_the_flowing_float_type_and_a_recursive_result_the_calls_type()
 {
    let failures = flowing_float_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn where_the_flow_stops_a_literal_beside_a_fixed_type_operand_takes_the_operands_type() {
    let rows: &[support::Row] = &[
        (
            &["Clip c:\n    Note when u8(250) < 400\n"],
            &[(Code::DoesNotFit, "400")],
        ),
        (
            &["Clip c:\n    Note/([1, 2, 3][u8(2) + 300])\n"],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &[
                "fn r(n): if n > 0 then r(n - 1) else u8(250) + 300\n",
                "Clip c:\n    Note/([60][r(1) - 300])\n",
            ],
            &[(Code::DoesNotFit, "300")],
        ),
        (
            &["Clip c | gain=(16777217 / 2i32):\n    Note\n"],
            &[(Code::DoesNotFit, "16777217")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn an_annotated_value_holding_a_faulty_choice_draws_the_faults_inside_it_and_no_shape_mismatch_392()
{
    let read = "Clip c:\n    Note/(a)\n";
    let rows: &[support::Row] = &[
        (
            &["let a: i32 = if [1, 2][5] then [1, 2] else [1, 2]\n", read],
            &[
                (Code::ConditionType, "[1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &[
                "let a: i32[3] = if [1, 2][5] then [1, 2] else [1, 2]\n",
                "Clip c:\n    Note/(a[0])\n",
            ],
            &[
                (Code::ConditionType, "[1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &[
                "let a: i32 = if 1 + [1, 2][5] then [1, 2] else [1, 2]\n",
                read,
            ],
            &[
                (Code::ConditionType, "1 + [1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &["let a: i32 = if 1 then [1, 2] else [1, 2]\n", read],
            &[(Code::ConditionType, "1")],
        ),
        (
            &["let a: i32 = if [1, 2][5] then 1 else 2\n", read],
            &[
                (Code::ConditionType, "[1, 2][5]"),
                (Code::IndexOutOfRange, "5"),
            ],
        ),
        (
            &[
                "let a: i32 = if [1, 2][1] > 1 then [1, 2] else [1, 2]\n",
                read,
            ],
            &[(
                Code::TypeMismatch,
                "let a: i32 = if [1, 2][1] > 1 then [1, 2] else [1, 2]\n",
            )],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

const FLOAT_BODIES: [&str; 4] = [
    "u8(250) + 300",
    "16777216 + 1",
    "2 ** -1 + i32(3)",
    "(u8(200) + 100) * 3",
];

fn float_body_rows(rows: &Rows) -> Vec<String> {
    let compiler =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    let mut failures = Vec::new();
    for (form, body) in FLOAT_BODIES.iter().enumerate() {
        for (stand, place) in FLOAT_STANDS.iter().enumerate() {
            for (depth, n) in ["0", "1", "3"].into_iter().enumerate() {
                for reversed in [false, true] {
                    if !rows.take(&[form, stand, depth, usize::from(reversed)]) {
                        continue;
                    }
                    let read = place.replace("{x}", &format!("r({n})"));
                    let program = |function: String| {
                        if reversed {
                            format!("{read}{function}")
                        } else {
                            format!("{function}{read}")
                        }
                    };
                    let text = program(format!("fn r(n): if n > 0 then r(n - 1) else {body}\n"));
                    let twin = program(format!("fn r(n): {body}\n"));
                    let found = support::error_texts(&compiler, &text);
                    let expected = support::error_texts(&compiler, &twin);
                    if found != expected || !expected.is_empty() {
                        failures.push(format!(
                            "292: {text}\n  draws {found:?}\n  twin {expected:?}"
                        ));
                    }
                }
            }
        }
    }
    failures
}

#[test]
fn an_integer_looking_recursive_body_under_a_float_expected_type_computes_in_that_type_from_its_literals_up_292()
 {
    let failures = covered(float_body_rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_an_integer_looking_recursive_body_under_a_float_expected_type_computes_in_that_type_from_its_literals_up_292()
 {
    let failures = float_body_rows(&Rows::Every);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
