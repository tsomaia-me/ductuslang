mod support;

use symphonyscript::Code;

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
            "input n = 1.0\nClip c | gain=(n / 16777217)\n",
            &[Code::DoesNotFit],
        ),
        ("input n = 1.0\nOsc o | freq=(n / 16777217)\n", &[]),
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
        ("input n = 1.0\nClip c | gain=(n / (16777216 + 1))\n", &[]),
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
fn a_recursive_call_computes_flexible_integer_arguments_exactly() {
    let fact = "fn fact(n): if n < 1 then 1 else n * fact(n - 1)\n";
    for k in 0u32..=30 {
        let value: u128 = (1..=u128::from(k)).product();
        let exact = value as f64 as u128 == value;
        let expected: &[Code] = if exact { &[] } else { &[Code::DoesNotFit] };
        expect(&[(&format!("{fact}Osc o | freq=(fact({k}))\n"), expected)]);
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
            "input fill_on = false\ninput section = 0\nClip verse:\n    Note/60\n    when fill_on:\n        Note/62 Note/64\n    else:\n        Note/62\n    when:\n        section is 0: Note/67\n        section is 1: Note/69\n        else: Note/71\n",
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
            .filter(|diagnostic| diagnostic.code == Code::IntegerInput)
            .count()
    };
    assert_eq!(
        integer_inputs("input n = 4\nClip c:\n    Note when n > 0.5\n"),
        1
    );
    assert_eq!(
        integer_inputs("input n = 4\nClip c:\n    Note when n > 1\n"),
        0
    );
    assert_eq!(
        integer_inputs("input n = 4\nClip c:\n    Note when n + 1 > 0.5\n"),
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
        ("let y = 1 + true\n", &[Code::OperandType]),
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
            "input cutoff = 0.5\nClip m:\n    Note | brightness=cutoff * 0.8\n",
            &[],
        ),
        ("input n = 4\nClip c:\n    Note/(n div 2)\n", &[]),
        (
            "input n = 4\nClip c:\n    Note/(n / 2)\n",
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
        ("input a = 3000000000\nSynth s:\n    Osc | freq=a\n", &[]),
        (
            "input a = 3000000000\nClip c:\n    Note/a\n",
            &[Code::DoesNotFit],
        ),
        (
            "input fine: f32 = 0\nClip c:\n    Note | velocity=fine\n",
            &[],
        ),
        ("input a: i32 = 0.5\n", &[Code::TypeMismatch]),
        (
            "input a = 1\nlet b = a + 0.5\nClip c:\n    Note when b > 1\n",
            &[],
        ),
        (
            "input f = 0.5\nlet g = f * 2\nClip c:\n    Note/g\n",
            &[Code::FloatToInteger],
        ),
        ("input f = 0.5\nClip c:\n    Note/(i32(f))\n", &[]),
    ]);
}

#[test]
fn a_reactive_let_read_by_another_keeps_the_width_of_the_input_it_reads() {
    for links in 1..=50 {
        for (annotation, misfits) in [("", 0), (": i32", links)] {
            let mut text = format!("input a{annotation} = 1\nlet l1 = a + 1\n");
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
        ("let x: i32 = 0.5\n", &[Code::TypeMismatch]),
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
            "input b = true\nClip c:\n    Note/(if b then 1 else 3000000000)\n",
            &[Code::DoesNotFit],
        ),
        ("input i = 0\nClip c:\n    Note/([1, 2][i + 5])\n", &[]),
    ]);
}

#[test]
fn enums_strings_and_bools_take_their_own_types() {
    expect(&[
        ("Synth s:\n    Osc | wave=Saw\n", &[]),
        ("input w = Wave::Saw\nSynth s:\n    Osc | wave=w\n", &[]),
        ("Synth s:\n    Osc | wave=1\n", &[Code::TypeMismatch]),
        ("let w = Saw\nSynth s:\n    Osc | wave=w\n", &[]),
        (
            "Kit k:\n    for i in 0..2:\n        Sampler | file=\"kit/{i}.wav\"\n",
            &[],
        ),
        ("Kit k:\n    Sampler | file=1\n", &[Code::TypeMismatch]),
        ("input c = 0.5\nClip m:\n    Note | accent=(c > 0.5)\n", &[]),
    ]);
}

#[test]
fn a_bare_variant_stands_only_where_an_enum_is_expected() {
    expect(&[
        ("Clip c | gain=Saw(1)\n", &[Code::UnknownName]),
        ("let z = Saw.gain\nClip c | gain=z\n", &[Code::UnknownName]),
        (
            "input w = Saw\nSynth s:\n    Osc | wave=w\n",
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
        (
            "input w = Wave::Saw\nSynth s:\n    Osc when w is Saw\n",
            &[],
        ),
        (
            "input w = Wave::Saw\nlet level = match w:\n    Sine: 1.0\n    Saw: 0.8\n    else: 0.6\nClip c | gain=level\n",
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
    let held = "Session s\ninput f = true\nlet x = if f then Sine else Saw\n";
    let rows = [
        format!("{held}Clip c:\n    Note when x is Wave::Saw\n"),
        format!("{held}Clip c:\n    Note when x is not Wave::Saw\n"),
        format!("{held}Clip c:\n    Note when Wave::Saw is x\n"),
        format!("{held}input w = Wave::Saw\nClip c:\n    Note when x is w\n"),
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
    let held = "Session s\ninput f = true\nlet x = if f then Sine else Saw\n";
    let the_let = "let x = if f then Sine else Saw\n";
    for uses in [
        "Clip c:\n    Note when x is x\n",
        "Clip c:\n    Note when x is Saw\n",
        "Clip c:\n    Note/x\n",
        "Clip c:\n    Note/(x + 1)\n",
    ] {
        support::expect_codes(
            &format!("{held}{uses}"),
            &[(Code::TypeMismatch, the_let, 0)],
        );
    }
    support::expect_codes(
        "input f = true\nlet x = if f then \"a\" else \"b\"\nClip c:\n    Note when x is \"a\"\n",
        &[(
            Code::ReactiveString,
            "let x = if f then \"a\" else \"b\"\n",
            0,
        )],
    );
    support::expect_codes(
        &format!("{held}let y = x\nClip c:\n    Note when y is 1\n"),
        &[(Code::TypeMismatch, the_let, 0)],
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
    let held = "Session s\ninput f = true\ninput g = true\nlet x = if f then Sine else Saw\n";
    let the_let = "let x = if f then Sine else Saw\n";
    let other = "let w = if f then Saw else Square\n";
    let failures: Vec<String> = [
        (
            "let y = x\n",
            "    Note when y + true\n",
            vec![(the_let, vec!["let y"])],
        ),
        (
            "let y = x\nlet z = y\n",
            "    Note when z + true\n",
            vec![(the_let, vec!["let y", "let z"])],
        ),
        (
            "let y = x\n",
            "    Note when x + true\n    Note when y + true\n",
            vec![(the_let, vec!["let y"])],
        ),
        (
            "let y = x\nlet z = x\n",
            "    Note when y + true\n    Note when z + true\n",
            vec![(the_let, vec!["let y", "let z"])],
        ),
        (
            "let w = if f then Saw else Square\nlet y = if g then x else w\n",
            "    Note when y + true\n",
            vec![(the_let, vec![other, "let y"])],
        ),
        (
            "let y = if f then x else Square\n",
            "    Note when y + true\n",
            vec![(the_let, vec!["let y"])],
        ),
        (
            "let w = if f then Saw else Square\n",
            "    Note when x is w\n",
            vec![(the_let, vec![other])],
        ),
        (
            "let w = if f then Saw else Square\n",
            "    Note when x + true\n    Note when w + true\n",
            vec![(the_let, vec![]), (other, vec![])],
        ),
        ("", "    Note when x + true\n", vec![(the_let, vec![])]),
    ]
    .into_iter()
    .filter_map(|(lets, uses, expected)| {
        let text = format!("{held}{lets}Clip c:\n{uses}");
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
                        .secondary
                        .iter()
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
    let held = "Session s\ninput f = true\nlet x = if f then Sine else Saw\n";
    let typed = "Synth s2:\n    Osc | wave=x\n";
    support::expect_codes(
        "Session s\ninput w = Wave::Saw\nlet y = w\nClip c:\n    Note when y\n",
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
    support::expect_codes(
        &format!("{held}let y = x\nClip c:\n    Note when y + true\n"),
        &[(Code::TypeMismatch, "let x = if f then Sine else Saw\n", 0)],
    );
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
            "input d = 5ms\nlet e = d * 2\nClip c:\n    Note | length=e\n",
            &[Code::UnitMismatch],
        ),
        ("Clip c:\n    Note | length=(2bars / 1bars)\n", &[]),
        ("Synth s:\n    Osc | freq=(1s / 1ms)\n", &[]),
    ]);
}
