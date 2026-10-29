mod support;

use symphonyscript::{Code, Compiler, Diagnostic};
use symphonyscript_core::Limits;
use symphonyscript_fixtures::vocabulary;

fn spans(text: &str, diagnostics: &[Diagnostic]) -> Vec<(Code, String, Vec<String>)> {
    let piece = |range: &std::ops::Range<u32>| {
        text.get(range.start as usize..range.end as usize)
            .unwrap_or_default()
            .to_string()
    };
    diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.code,
                piece(&diagnostic.span.range),
                diagnostic
                    .related
                    .iter()
                    .map(|related| &related.span)
                    .map(|span| piece(&span.range))
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn an_error_in_an_inlined_body_names_its_calls_innermost_first() {
    let fit = |call: &str| {
        (
            Code::DoesNotFit,
            "n * 2".to_string(),
            vec![call.to_string()],
        )
    };
    let loud = "fn loud(n): n * 2\nClip c:\n";
    let rows = [
        (
            format!(
                "{loud}    Note/(loud(2000000000))\n    Note/60\n    Note/(loud(1500000000))\n"
            ),
            vec![fit("loud(2000000000)"), fit("loud(1500000000)")],
        ),
        (
            format!("{loud}    Note/(loud(2000000000))\n    Note/(loud(2000000000))\n"),
            vec![fit("loud(2000000000)"), fit("loud(2000000000)")],
        ),
        (
            format!("{loud}    Note/(loud(2000000000))\n    Note/(loud(5))\n"),
            vec![fit("loud(2000000000)")],
        ),
    ];
    for (text, expected) in rows {
        let found = spans(&text, &support::checked(&text));
        assert_eq!(found, expected, "{text}");
    }
    let nested = "fn loud(n): n * 2\nfn twice(m): loud(m)\nClip c:\n    Note/(twice(2000000000))\n";
    assert_eq!(
        spans(nested, &support::checked(nested)),
        vec![(
            Code::DoesNotFit,
            "n * 2".to_string(),
            vec!["loud(m)".to_string(), "twice(2000000000)".to_string()]
        )]
    );
}

fn doubled_patterns(levels: usize, uses: usize) -> String {
    let mut text = String::from("pattern P0():\n    Note/300000000000\n");
    for level in 1..levels {
        let below = level - 1;
        text.push_str(&format!(
            "pattern P{level}():\n    P{below}\n    P{below}\n"
        ));
    }
    text.push_str("Clip c:\n");
    for _ in 0..uses {
        text.push_str(&format!("    P{}\n", levels - 1));
    }
    text
}

#[test]
fn a_fault_draws_one_diagnostic_per_outermost_call_or_use() {
    let fit = |chain: &[&str]| {
        (
            Code::DoesNotFit,
            "300000000000".to_string(),
            chain
                .iter()
                .map(|call| call.to_string())
                .collect::<Vec<_>>(),
        )
    };
    let once = doubled_patterns(12, 1);
    let uses: Vec<String> = (0..12).map(|level| format!("P{level}")).collect();
    let chain: Vec<&str> = uses.iter().map(String::as_str).collect();
    assert_eq!(spans(&once, &support::checked(&once)), vec![fit(&chain)]);
    let twice = doubled_patterns(12, 2);
    let found = spans(&twice, &support::checked(&twice));
    assert_eq!(found, vec![fit(&chain), fit(&chain)]);
    let positions: Vec<Option<u32>> = support::checked(&twice)
        .iter()
        .map(|diagnostic| {
            diagnostic
                .related
                .last()
                .map(|related| &related.span)
                .map(|span| span.range.start)
        })
        .collect();
    assert_ne!(positions.first(), positions.last());
    let out_of_range = |chain: &[&str]| {
        (
            Code::IndexOutOfRange,
            "n".to_string(),
            chain
                .iter()
                .map(|call| call.to_string())
                .collect::<Vec<_>>(),
        )
    };
    let shared_inner =
        "fn pick(n): [1, 2][n]\nfn both(x): pick(x) + pick(x)\nClip c:\n    Note/(both(5))\n";
    let diagnostics = support::checked(shared_inner);
    assert_eq!(
        spans(shared_inner, &diagnostics),
        vec![out_of_range(&["pick(x)", "both(5)"])]
    );
    let first_pick = shared_inner
        .find("pick(x)")
        .and_then(|at| u32::try_from(at).ok());
    assert_eq!(
        diagnostics
            .first()
            .and_then(|diagnostic| diagnostic.related.first().map(|related| &related.span))
            .map(|span| span.range.start),
        first_pick
    );
    let siblings = "fn pick(n): [1, 2][n]\nClip c:\n    Note/(pick(5) + pick(5))\n";
    let found = spans(siblings, &support::checked(siblings));
    assert_eq!(
        found,
        vec![out_of_range(&["pick(5)"]), out_of_range(&["pick(5)"])]
    );
    let starts: Vec<Option<u32>> = support::checked(siblings)
        .iter()
        .map(|diagnostic| {
            diagnostic
                .related
                .first()
                .map(|related| &related.span)
                .map(|span| span.range.start)
        })
        .collect();
    assert_ne!(starts.first(), starts.last());
}

type Sites<'a> = &'a [(Code, &'a str, &'a str, usize)];

#[test]
fn a_fault_in_a_recursive_body_draws_one_diagnostic_per_outermost_call_or_use() {
    let overflow = "2147483647i32 + 1i32";
    let f = "fn f(n): if n < 1 then 2147483647i32 + 1i32 else f(n - 1)\n";
    let pick = "fn pick(n): if n < 1 then [1, 2][5] else pick(n - 1)\n";
    let g = "fn g(n): if n < 1 then 1 div 0.5 else g(n - 1)\n";
    let fib = "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)\n";
    let rows: [(&[&str], Sites); 6] = [
        (
            &[
                f,
                pick,
                "Clip c:\n    Note/(f(1))\n    Note/60\n    Note/(f(1))\n    Note/(f(2))\n    Note/(pick(1))\n    Note/(pick(1))\n",
            ],
            &[
                (Code::ConstantOverflow, overflow, "f(1)", 0),
                (Code::ConstantOverflow, overflow, "f(1)", 1),
                (Code::ConstantOverflow, overflow, "f(2)", 0),
                (Code::IndexOutOfRange, "5", "pick(1)", 0),
                (Code::IndexOutOfRange, "5", "pick(1)", 1),
            ],
        ),
        (
            &[
                f,
                "pattern P():\n    Note/(f(1))\n",
                "Clip c:\n    P\n    P\n",
            ],
            &[
                (Code::ConstantOverflow, overflow, "P\n", 0),
                (Code::ConstantOverflow, overflow, "P\n", 1),
            ],
        ),
        (
            &[
                g,
                "Clip c:\n    Note/(g(1))\n    Note/(g(1))\n    Note/(g(2))\n",
            ],
            &[
                (Code::OperandType, "1 div 0.5", "g(1)", 0),
                (Code::OperandType, "1 div 0.5", "g(1)", 1),
                (Code::OperandType, "1 div 0.5", "g(2)", 0),
            ],
        ),
        (
            &[f, "Clip c:\n    Note/(f(1))\n    Note/60\n"],
            &[(Code::ConstantOverflow, overflow, "f(1)", 0)],
        ),
        (
            &[
                fib,
                "Clip c:\n    Note/(fib(10))\n    Note/(fib(10))\n    Note/(fib(12))\n",
            ],
            &[],
        ),
        (
            &[
                "fn loud(n): n * 2\n",
                "Clip c:\n    Note/(loud(2000000000))\n    Note/(loud(2000000000))\n",
            ],
            &[
                (Code::DoesNotFit, "n * 2", "loud(2000000000)", 0),
                (Code::DoesNotFit, "n * 2", "loud(2000000000)", 1),
            ],
        ),
    ];
    for (items, sites) in rows {
        for order in support::permutations(items) {
            let text = order.concat();
            let mut expected: Vec<(Code, String, Option<u32>)> = sites
                .iter()
                .map(|(code, fault, site, occurrence)| {
                    (
                        *code,
                        fault.to_string(),
                        Some(support::at(&text, site, *occurrence).0),
                    )
                })
                .collect();
            expected.sort();
            let mut found: Vec<(Code, String, Option<u32>)> = support::checked(&text)
                .iter()
                .map(|diagnostic| {
                    let range = &diagnostic.span.range;
                    (
                        diagnostic.code,
                        text.get(range.start as usize..range.end as usize)
                            .unwrap_or_default()
                            .to_string(),
                        diagnostic
                            .related
                            .last()
                            .map(|related| related.span.range.start),
                    )
                })
                .collect();
            found.sort();
            assert_eq!(found, expected, "{text}");
        }
    }
}

type Labels<'a> = &'a [(&'a str, &'a str)];

type Recursion<'a> = (&'a [&'a str], &'a str, &'a [&'a str], &'a str, Labels<'a>);

fn labelled(text: &str, diagnostic: &Diagnostic) -> Vec<(String, String)> {
    diagnostic
        .related
        .iter()
        .map(|related| {
            let range = &related.span.range;
            (
                text.get(range.start as usize..range.end as usize)
                    .unwrap_or_default()
                    .to_string(),
                related.label.clone(),
            )
        })
        .collect()
}

fn a_call_that_a_bounded_expansion_repeats_is_named_once_with_its_levels_in(
    orders: support::Orders,
) {
    let input = "input n = 1\n";
    let deep = "fn f(x) bounded 2000000 else 0: if x > 3 then x else f(x + 1)\n";
    let use_f = "Clip c:\n    Note/(f(n))\n";
    let limited = |compiler: &Compiler, text: &str, levels: &str| {
        let found = support::errors(compiler.check(&[support::source(text)]));
        let [diagnostic] = found.as_slice() else {
            panic!("{text}{found:?}");
        };
        assert_eq!(diagnostic.code, Code::InlineLimit, "{text}");
        assert_eq!(
            labelled(text, diagnostic),
            vec![
                (
                    "f(x + 1)".to_string(),
                    format!("in this call, repeated over {levels} levels")
                ),
                ("f(n)".to_string(), "in this call".to_string()),
            ],
            "{text}"
        );
    };
    let ten_thousand = Compiler::with_limits(
        &vocabulary(),
        Limits {
            inlined_calls: 10_000,
            ..Limits::default()
        },
    )
    .expect("the vocabulary compiles");
    limited(&ten_thousand, &format!("{input}{deep}{use_f}"), "9,999");
    let thousand = Compiler::with_limits(
        &vocabulary(),
        Limits {
            inlined_calls: 1000,
            ..Limits::default()
        },
    )
    .expect("the vocabulary compiles");
    for order in orders.of(&[input, deep, use_f]) {
        limited(&thousand, &order.concat(), "999");
    }
    let every_level = "fn g(x) bounded 5 else 0: x div 0.5 + g(x + 1)\n";
    let wrapped = "fn k(x): g(x) + 1\n";
    let rows: &[(&[&str], Labels)] = &[
        (
            &[input, every_level, "Clip c:\n    Note/(g(n))\n"],
            &[
                ("g(x + 1)", "in this call, repeated over 4 levels"),
                ("g(n)", "in this call"),
            ],
        ),
        (
            &[input, every_level, wrapped, "Clip c:\n    Note/(k(n))\n"],
            &[
                ("g(x + 1)", "in this call, repeated over 4 levels"),
                ("g(x)", "in this call"),
                ("k(n)", "in this call"),
            ],
        ),
        (
            &[
                input,
                "fn h(x): x div 0.5\n",
                "fn w(x): h(x) + 1\n",
                "Clip c:\n    Note/(w(n))\n",
            ],
            &[("h(x)", "in this call"), ("w(n)", "in this call")],
        ),
    ];
    for (items, expected) in rows {
        for order in orders.of(items) {
            let text = order.concat();
            let found = support::checked(&text);
            let [diagnostic] = found.as_slice() else {
                panic!("{text}{found:?}");
            };
            assert_eq!(diagnostic.code, Code::OperandType, "{text}");
            let expected: Vec<(String, String)> = expected
                .iter()
                .map(|(call, label)| (call.to_string(), label.to_string()))
                .collect();
            assert_eq!(labelled(&text, diagnostic), expected, "{text}");
        }
    }
}

#[test]
fn a_call_that_a_bounded_expansion_repeats_is_named_once_with_its_levels() {
    a_call_that_a_bounded_expansion_repeats_is_named_once_with_its_levels_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_call_that_a_bounded_expansion_repeats_is_named_once_with_its_levels() {
    a_call_that_a_bounded_expansion_repeats_is_named_once_with_its_levels_in(
        support::Orders::Every,
    );
}

#[test]
fn reactive_recursion_names_its_chain_of_calls_and_the_functions_that_lack_a_bound() {
    let input = "input n = 1\n";
    let a = "fn a(x) bounded 3 else 0: b(x)\n";
    let rows: &[Recursion] = &[
        (
            &[
                input,
                a,
                "fn b(x): if x > 0 then a(x - 1) else 0\n",
                "Clip c:\n    Note/(a(n))\n",
            ],
            "a(n)",
            &["`a` → `b` → `a`", "`b` has none"],
            "declare a bound on `b`, written `bounded N else fallback`, as in `bounded 10 else 0`",
            &[
                ("b(x)", "`a` calls `b` here"),
                ("a(x - 1)", "`b` calls `a` here"),
            ],
        ),
        (
            &[
                input,
                "fn f(x): if x > 0 then f(x - 1) else 0\n",
                "Clip c:\n    Note/(f(n))\n",
            ],
            "f(n)",
            &["`f` → `f`", "`f` has none"],
            "declare a bound on `f`, written `bounded N else fallback`, as in `bounded 10 else 0`",
            &[("f(x - 1)", "`f` calls `f` here")],
        ),
        (
            &[
                input,
                "fn fib(k): if k < 2 then k else fib(k - 1) + fib(k - 2)\n",
                "Clip c:\n    Note/(fib(n))\n",
            ],
            "fib(n)",
            &["`fib` → `fib`", "`fib` has none"],
            "declare a bound on `fib`, written `bounded N else fallback`, as in `bounded 10 else 0`",
            &[("fib(k - 1)", "`fib` calls `fib` here")],
        ),
        (
            &[
                "fn g(x): if x > 0 then g(x - 1) else n\n",
                input,
                "Clip c:\n    Note/(g(3))\n",
            ],
            "g(3)",
            &[
                "the body of the recursive function `g` reads a cell",
                "`g` → `g`",
            ],
            "declare a bound on `g`, written `bounded N else fallback`, as in `bounded 10 else 0`",
            &[("g(x - 1)", "`g` calls `g` here")],
        ),
    ];
    for (items, at, words, help, chain) in rows {
        for order in support::permutations(items) {
            let text = order.concat();
            let found = support::checked(&text);
            let [diagnostic] = found.as_slice() else {
                panic!("{text}{found:?}");
            };
            let range = &diagnostic.span.range;
            assert_eq!(
                (
                    diagnostic.code,
                    &text[range.start as usize..range.end as usize]
                ),
                (Code::ReactiveRecursion, *at),
                "{text}"
            );
            for word in *words {
                assert!(diagnostic.summary.contains(word), "{text}{diagnostic:?}");
            }
            assert_eq!(diagnostic.help.as_deref(), Some(*help), "{text}");
            let expected: Vec<(String, String)> = chain
                .iter()
                .map(|(call, label)| (call.to_string(), label.to_string()))
                .collect();
            assert_eq!(labelled(&text, diagnostic), expected, "{text}");
        }
    }
    support::assert_rows_in_every_order(
        &vocabulary(),
        &[(
            &[
                input,
                a,
                "fn b(x) bounded 3 else 0: if x > 0 then a(x - 1) else 0\n",
                "Clip c:\n    Note/(a(n))\n",
            ],
            &[],
        )],
    );
}

#[test]
fn an_unbounded_call_whose_body_reads_a_cell_through_a_local_let_draws_reactive_recursion() {
    let call = "Clip c:\n    Note/(lowest(0))\n";
    let rows: &[support::Row] = &[
        (
            &[
                "input held = [1, 2, 3]\n",
                "fn lowest(i):\n    let a = held[i]\n    if i >= 3 then 127 else min(a, lowest(i + 1))\n",
                call,
            ],
            &[(Code::ReactiveRecursion, "lowest(0)")],
        ),
        (
            &[
                "input held: i32[..10] = []\n",
                "fn lowest(i):\n    let a = held[i]\n    if i >= len(held) then 127 else min(a, lowest(i + 1))\n",
                call,
            ],
            &[(Code::ReactiveRecursion, "lowest(0)")],
        ),
        (
            &[
                "input held = [1, 2, 3]\n",
                "fn lowest(i)  10 else 127:\n    let a = held[i]\n    if i >= 3 then 127 else min(a, lowest(i + 1))\n",
                call,
            ],
            &[(Code::FnSyntax, "10")],
        ),
        (
            &[
                "input held: i32[..10] = []\n",
                "fn lowest(i) bounded 10 else 127:\n    let a = held[i]\n    if i >= len(held) then 127 else min(a, lowest(i + 1))\n",
                call,
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn every_element_of_a_for_iterable_is_computed_read_or_not() {
    let rows: &[support::Row] = &[
        (
            &["Clip c:\n    for i in [[1, 2][5] + 1]:\n        Note\n"],
            &[(Code::IndexOutOfRange, "5")],
        ),
        (
            &["Clip c:\n    for i in [2 ** 200 + 1]:\n        Note\n"],
            &[(Code::IntegerOverflow, "2 ** 200")],
        ),
        (
            &[
                "fn f(n): if n < 1 then 0 else f(n - 1) + 1\n",
                "Clip c:\n    for i in [f(2000) + 1]:\n        Note\n",
            ],
            &[(Code::RecursionDepth, "f(n - 1)")],
        ),
        (
            &[
                "fn f(): f()\n",
                "Clip c:\n    for i in [f() + 1]:\n        Note\n",
            ],
            &[(Code::RecursionDepth, "f()")],
        ),
        (
            &[
                "fn f(): f()\n",
                "fn g(x): x + 1\n",
                "Clip c:\n    for i in [g(f())]:\n        Note\n",
            ],
            &[(Code::RecursionDepth, "f()")],
        ),
        (
            &[
                "fn g(x): x + 1\n",
                "Clip c:\n    for i in [1, 2]:\n        Note\n    for i in [g(3)]:\n        Note\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn an_endless_recursion_draws_one_recursion_depth() {
    let rows: &[support::Row] = &[
        (
            &[
                "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - -1)\n",
                "Clip c:\n    Note/(fib(6))\n",
            ],
            &[(Code::RecursionDepth, "fib(n - -1)")],
        ),
        (
            &["fn g(n): g(n + 1)\n", "Clip c:\n    Note/(g(0))\n"],
            &[(Code::RecursionDepth, "g(n + 1)")],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_bounded_functions_fallback_is_type_checked_against_its_body_at_every_call() {
    let depth = |fallback: &str| {
        format!(
            "fn depth(k) bounded 3 else {fallback}:\n    if k < 1 then 0 else 1 + depth(k - 1)\n"
        )
    };
    let pair = |fallback: &str| {
        format!(
            "fn pair(k) bounded 3 else {fallback}:\n    if k < 1 then [k, k] else pair(k - 1)\n"
        )
    };
    let lowest = |fallback: &str| {
        format!(
            "fn lowest(i) bounded 10 else {fallback}:\n    if i >= len(held) then 127 else min(held[i], lowest(i + 1))\n"
        )
    };
    let (list, boolean, variant) = (depth("[0, 1]"), depth("true"), depth("Wave::Saw"));
    let (parameters, zero, parameter) = (depth("[k, k]"), depth("0"), depth("k"));
    let (rows_fallback, longer) = (pair("[[0, 0]]"), pair("[0, 0, 0]"));
    let (held_list, held_scalar) = (lowest("[127]"), lowest("127"));
    let n = "input n = 2\n";
    let held = "input held: i32[..10] = []\n";
    let shallow = "Clip c:\n    Note/(60 + depth(2))\n";
    let deep = "Clip c:\n    Note/(60 + depth(5))\n";
    let reactive = "Clip c:\n    Note/(60 + depth(n))\n";
    let lowest_call = "Clip c:\n    Note/(lowest(0))\n";
    let pair_call = "Clip c:\n    Note/(pair(1)[0])\n";
    let rows: &[support::Row] = &[
        (&[&list, shallow], &[(Code::TypeMismatch, "[0, 1]")]),
        (
            &[
                "let v = if true then 0 else [0, 1]\n",
                "Clip c:\n    Note/(v)\n",
            ],
            &[(Code::TypeMismatch, "if true then 0 else [0, 1]")],
        ),
        (&[&list, deep], &[(Code::TypeMismatch, "[0, 1]")]),
        (&[&list, n, reactive], &[(Code::TypeMismatch, "[0, 1]")]),
        (&[&boolean, n, reactive], &[(Code::TypeMismatch, "true")]),
        (&[&boolean, shallow], &[(Code::TypeMismatch, "true")]),
        (
            &[&variant, n, reactive],
            &[(Code::TypeMismatch, "Wave::Saw")],
        ),
        (&[&variant, shallow], &[(Code::TypeMismatch, "Wave::Saw")]),
        (&[&parameters, shallow], &[(Code::TypeMismatch, "[k, k]")]),
        (
            &[&parameters, n, reactive],
            &[(Code::TypeMismatch, "[k, k]")],
        ),
        (
            &[&rows_fallback, pair_call],
            &[(Code::TypeMismatch, "[[0, 0]]")],
        ),
        (
            &[held, &held_list, lowest_call],
            &[(Code::TypeMismatch, "[127]")],
        ),
        (&[&zero, shallow], &[]),
        (&[&zero, n, reactive], &[]),
        (&[&parameter, n, reactive], &[]),
        (&[&longer, pair_call], &[]),
        (&[held, &held_scalar, lowest_call], &[]),
        (
            &[
                "fn r(n) bounded 3 else []:\n    if n < 1 then [[0, 0]] else r(n - 1) + [[n, n]]\n",
                "input m = 2\n",
                "Clip c:\n    Note/(len(r(2)) + len(r(m)))\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&symphonyscript_fixtures::vocabulary(), rows);
}

#[test]
fn a_range_bound_or_index_holding_a_recursive_call_evaluates_the_call_before_judging_its_kind() {
    let q = "fn q(n): if n < 1 then 0 else q(n - 1)\n";
    let operands = |call: &str, code: Code| {
        vec![
            (code, "n < 1".to_string(), vec![call.to_string()]),
            (code, "n - 1".to_string(), vec![call.to_string()]),
        ]
    };
    let depth =
        |at: &str, call: &str| vec![(Code::RecursionDepth, at.to_string(), vec![call.to_string()])];
    type Row<'a> = (Vec<&'a str>, Vec<(Code, String, Vec<String>)>);
    let arrays = operands("q([1, 2])", Code::ArrayOperand);
    let rows: Vec<Row> = vec![
        (vec![q, "Clip c:\n    Note/(q([1, 2]))\n"], arrays.clone()),
        (
            vec![q, "Clip c:\n    for i in 0..q([1, 2]):\n        Note\n"],
            arrays.clone(),
        ),
        (
            vec![q, "Clip c:\n    for i in q([1, 2])..3:\n        Note\n"],
            arrays.clone(),
        ),
        (
            vec![q, "Clip c:\n    for i in q([1, 2]):\n        Note\n"],
            arrays.clone(),
        ),
        (
            vec![q, "Clip c:\n    [for i in 0..q([1, 2]): Note]\n"],
            arrays.clone(),
        ),
        (
            vec![q, "Clip c:\n    Note/([1, 2][q([1, 2])])\n"],
            arrays.clone(),
        ),
        (
            vec![q, "Clip c:\n    Note/([1, 2][^q([1, 2])])\n"],
            arrays.clone(),
        ),
        (
            vec![q, "Clip c:\n    for i in 0..q(true):\n        Note\n"],
            operands("q(true)", Code::OperandType),
        ),
        (
            vec![
                "fn f(n): f(n + 1)\n",
                "Clip c:\n    for i in 0..f(0):\n        Note\n",
            ],
            depth("f(n + 1)", "f(0)"),
        ),
        (
            vec![
                "fn f(): f()\n",
                "Clip c:\n    for i in 0..f():\n        Note\n",
            ],
            depth("f()", "f()"),
        ),
        (
            vec!["fn f(n): f(n + 1)\n", "Clip c:\n    Note/([1, 2][f(0)])\n"],
            depth("f(n + 1)", "f(0)"),
        ),
        (
            vec![
                "fn g(n): if n < 1 then [1, 2] else g(n + 1)\n",
                "Clip c:\n    for j in g(1):\n        Note\n",
            ],
            depth("g(n + 1)", "g(1)"),
        ),
        (
            vec![
                q,
                "Clip c:\n    for i in 0..q(2):\n        Note\n    Note/([5, 6, 7][q(2)])\n",
            ],
            vec![],
        ),
        (
            vec!["Clip c:\n    for i in 0..2.5:\n        Note\n"],
            vec![(Code::RangeBound, "2.5".to_string(), vec![])],
        ),
    ];
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    for (items, mut expected) in rows {
        expected.sort();
        for order in support::permutations(&items) {
            let text = order.concat();
            let mut found = spans(
                &text,
                &support::errors(compiler.check(&[support::source(&text)])),
            );
            found.sort();
            assert_eq!(found, expected, "{text}");
        }
    }
}

#[test]
fn an_operator_over_recursive_calls_takes_the_shapes_their_results_settle_to() {
    let doubling = "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n";
    let growing = "fn g(n): if n is 0 then [1] else g(n - 1) + [n]\n";
    let bounded = "fn f(n) bounded 3 else [1]: f(n - 1) + f(n - 1)\n";
    let held_input = "input i = 2\ninput held: i32[..4] = []\n";
    let doubled_held = "fn h(n) bounded 3 else held: h(n - 1) + h(n - 1)\n";
    let read_one = "Clip c:\n    Note/(60 + len(f(1)))\n";
    let rows: &[support::Row] = &[
        (&[doubling, "Clip c:\n    Note/(60 + len(f(3)))\n"], &[]),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) - f(n - 1)\n",
                read_one,
            ],
            &[(Code::ArrayOperand, "f(n - 1) - f(n - 1)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) * f(n - 1)\n",
                read_one,
            ],
            &[(Code::ArrayOperand, "f(n - 1) * f(n - 1)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) div f(n - 1)\n",
                read_one,
            ],
            &[(Code::ArrayOperand, "f(n - 1) div f(n - 1)")],
        ),
        (
            &["fn f(n): if n is 0 then [1] else f(n - 1) - 1\n", read_one],
            &[(Code::ArrayOperand, "f(n - 1) - 1")],
        ),
        (
            &["Clip c:\n    Note/(60 + len([1] - [1]))\n"],
            &[(Code::ArrayOperand, "[1] - [1]")],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note when f(2) < f(1)\n",
            ],
            &[(Code::ArrayOperand, "f(2) < f(1)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note when len(f(2)) > len(f(1))\n",
            ],
            &[],
        ),
        (&[growing, "Clip c:\n    Note/(60 + g(3)[3])\n"], &[]),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + g(n)\n",
                "fn g(n): if n is 0 then [2] else f(n - 1)\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            &[],
        ),
        (&[bounded, "Clip c:\n    Note/(60 + len(f(0)))\n"], &[]),
        (
            &[
                "fn f(n) bounded 3 else [1]: f(n - 1) + f(n - 2)\n",
                "Clip c:\n    Note/(60 + len(f(0)))\n",
            ],
            &[],
        ),
        (
            &[
                "input i = 2\n",
                bounded,
                "Clip c:\n    Note/(60 + len(f(i)))\n",
            ],
            &[],
        ),
        (
            &[
                "input i = 2\n",
                bounded,
                "let p: i32[8] = f(i)\n",
                "Clip c:\n    Note/(p[7])\n",
            ],
            &[],
        ),
        (
            &[
                "input i = 2\n",
                bounded,
                "let n: u8 = len(f(i))\n",
                "Clip c:\n    Note/(n)\n",
            ],
            &[],
        ),
        (
            &[held_input, doubled_held, "Clip c:\n    Note/(len(h(i)))\n"],
            &[],
        ),
        (
            &[
                held_input,
                doubled_held,
                "let q: i32[..32] = h(i)\n",
                "Clip c:\n    Note/(len(q))\n",
            ],
            &[],
        ),
        (
            &[
                held_input,
                doubled_held,
                "let n: u8 = len(h(i))\n",
                "Clip c:\n    Note/(n)\n",
            ],
            &[(Code::TypeMismatch, "len(h(i))")],
        ),
        (
            &[
                "fn s(n): if n is 0 then 1 else s(n - 1) + s(n - 1)\n",
                "Clip c:\n    Note/(60 + s(3))\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n) bounded 3 else 0: f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note/(60 + f(0))\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + 1\n",
                "Clip c:\n    Note/(len(f(2)))\n",
            ],
            &[(Code::ArrayOperand, "f(n - 1)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else [2] + f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note/(60 + len(f(3)))\n",
            ],
            &[],
        ),
        (
            &[
                "fn h(n): [n, n]\n",
                "fn f(n): if n is 0 then [1] else f(n - 1) + h(n)\n",
                "Clip c:\n    Note/(60 + len(f(3)))\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + [len(f(n - 1))]\n",
                "Clip c:\n    Note/(60 + f(3)[3])\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [5] else [f(n - 1)[^1] + 1]\n",
                "Clip c:\n    Note/(60 + f(3)[0])\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1, 2] else (f(n - 1) + f(n - 1))[1..]\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + [len(bytes(f(n - 1)) + [n])]\n",
                "Clip c:\n    Note/(60 + f(2)[2])\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [300] else bytes(f(n - 1)) + [n]\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            &[],
        ),
        (
            &[growing, "Clip c:\n    Note/(60 + len(g(2) + g(1)))\n"],
            &[],
        ),
        (
            &[
                growing,
                "let a = g(2) + g(1)\n",
                "Clip c:\n    Note/(60 + a[2])\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [[1, 2]] else f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note/(60 + f(2)[3][1])\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [[1, 2]] else [f(n - 1)[0] + f(n - 1)[0]]\n",
                "Clip c:\n    Note/(60 + len(f(2)[0]))\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then [true] else f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note when f(2)[3]\n",
            ],
            &[],
        ),
        (
            &[
                "fn f(n): if n is 0 then true else f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note when f(2)\n",
            ],
            &[(Code::OperandType, "f(n - 1) + f(n - 1)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + g(n)\n",
                "fn g(n): if n is 0 then 2 else g(n - 1)\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            &[(Code::ArrayOperand, "f(n - 1) + g(n)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then 1 else f(n - 1) + [n]\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            &[(Code::ArrayOperand, "f(n - 1) + [n]")],
        ),
        (
            &[doubling, "Clip c:\n    Note/(f(2) + f(2))\n"],
            &[(Code::TypeMismatch, "f(2) + f(2)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then 1 else len(f(n - 1))\n",
                "Clip c:\n    Note/(f(2))\n",
            ],
            &[(Code::NotAnArray, "f(n - 1)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then 1 else len(f(n - 1)[1..])\n",
                "Clip c:\n    Note/(f(2))\n",
            ],
            &[(Code::NotAnArray, "f(n - 1)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then 1 else len(bytes(f(n - 1)))\n",
                "Clip c:\n    Note/(f(2))\n",
            ],
            &[(Code::OperandType, "f(n - 1)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then 1 else f(n - 1)\n",
                "Clip c:\n    Note/(len(f(2)[1..]))\n",
            ],
            &[(Code::NotAnArray, "f(2)")],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let narrow = [
        held_input,
        doubled_held,
        "let q: i32[..31] = h(i)\n",
        "Clip c:\n    Note/(len(q))\n",
    ];
    for order in support::permutations(&narrow) {
        let text = order.concat();
        let found = support::errors(compiler.check(&[support::source(&text)]));
        let summaries: Vec<(Code, &str)> = found
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.summary.as_str()))
            .collect();
        assert_eq!(
            summaries,
            [(
                Code::CapacityExceeded,
                "`q` holds up to 32 elements, more than its annotated capacity 31"
            )],
            "{text}"
        );
    }
}

#[test]
fn every_join_the_evaluation_performs_compares_the_shapes_of_its_rows() {
    let appending = "fn f(n) bounded 3 else [[1, 2]]: f(n - 1) + [[3]]\n";
    let unbounded = "fn f(n): if n is 0 then [[1, 2]] else f(n - 1) + [[3]]\n";
    let ragged = &[(Code::RaggedArray, "f(n - 1) + [[3]]")];
    let rows: &[support::Row] = &[
        (&[appending, "Clip c:\n    Note/(60 + len(f(0)))\n"], ragged),
        (
            &[
                "fn f(n) bounded 3 else [[1, 2]]: [[3]] + f(n - 1)\n",
                "Clip c:\n    Note/(60 + len(f(0)))\n",
            ],
            &[(Code::RaggedArray, "[[3]] + f(n - 1)")],
        ),
        (&[unbounded, "Clip c:\n    Note/(60 + len(f(2)))\n"], ragged),
        (&[unbounded, "Clip c:\n    Note/(60 + len(f(3)))\n"], ragged),
        (
            &[
                "fn f(n) bounded 3 else [[[1, 2]]]: f(n - 1) + [[[3]]]\n",
                "Clip c:\n    Note/(60 + len(f(0)))\n",
            ],
            &[(Code::RaggedArray, "f(n - 1) + [[[3]]]")],
        ),
        (
            &[
                "fn f(n) bounded 3 else [[1, 2]]: f(n - 1) + g(n)\n",
                "fn g(n) bounded 3 else [[3]]: g(n - 1) + g(n - 1)\n",
                "Clip c:\n    Note/(60 + len(f(0)))\n",
            ],
            &[(Code::RaggedArray, "f(n - 1) + g(n)")],
        ),
        (
            &[
                "fn f(n): if n is 0 then [[1, 2]] else f(n - 1) + g(n)\n",
                "fn g(n): if n is 0 then [[1, 2, 3]] else g(n - 1)\n",
                "Clip c:\n    Note/(60 + len(f(1)))\n",
            ],
            &[(Code::RaggedArray, "f(n - 1) + g(n)")],
        ),
        (
            &[
                appending,
                "let p: i32[2][4] = f(0)\n",
                "Clip c:\n    Note/(p[0][0])\n",
            ],
            ragged,
        ),
        (
            &[
                "fn f(n): if n is 0 then [[1, 2]] else f(n - 1) + [[3, 4]]\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            &[],
        ),
        (
            &[
                "let x = [[1, 2]] + [[3]]\n",
                "Clip c:\n    Note/(x[0][0])\n",
            ],
            &[(Code::RaggedArray, "[[1, 2]] + [[3]]")],
        ),
        (
            &[
                "input i = 1\n",
                appending,
                "Clip c:\n    Note/(len(f(i)))\n",
            ],
            ragged,
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

#[test]
fn a_join_whose_length_would_reach_two_to_the_thirty_one_draws_length_limit_before_it_is_built() {
    let doubling = "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n";
    let limited = &[(Code::LengthLimit, "f(n - 1) + f(n - 1)")];
    let rows: &[support::Row] = &[
        (
            &[doubling, "Clip c:\n    Note/(60 + len(f(31)))\n"],
            limited,
        ),
        (
            &[doubling, "Clip c:\n    Note/(60 + len(f(40)))\n"],
            limited,
        ),
        (
            &[
                "fn f(n) bounded 31 else [1]: f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note/(60 + len(f(0)))\n",
            ],
            limited,
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

#[test]
fn a_join_whose_length_stays_below_two_to_the_thirty_one_draws_no_length_limit() {
    let doubling = "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n";
    let rows: &[support::Row] = &[
        (&[doubling, "Clip c:\n    Note/(60 + len(f(30)))\n"], &[]),
        (&[doubling, "Clip c:\n    Note/(60 + f(30)[5])\n"], &[]),
        (
            &[
                doubling,
                "let p: i32[8] = f(3)\n",
                "Clip c:\n    Note/(p[7])\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

const DOUBLING_ON_AN_INPUT: [&str; 3] = [
    "input i = 0\n",
    "fn f(n) bounded 31 else [1]: f(n - 1) + f(n - 1)\n",
    "Clip c:\n    Note/(len(f(i)))\n",
];

#[test]
#[ignore = "stress"]
fn stress_a_doubling_bounded_function_called_on_an_input_draws_inline_limit() {
    let reactive = DOUBLING_ON_AN_INPUT;
    let inline_limited = vec![(Code::InlineLimit, "f(n - 1)".to_string())];
    let defaults = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    assert_eq!(
        support::error_texts(&defaults, &reactive.concat()),
        inline_limited
    );
}

#[test]
fn a_lowered_inline_limit_refuses_a_doubling_bounded_function_on_an_input_in_every_order() {
    let reactive = DOUBLING_ON_AN_INPUT;
    let inline_limited = vec![(Code::InlineLimit, "f(n - 1)".to_string())];
    let thousand = Compiler::with_limits(
        &vocabulary(),
        Limits {
            inlined_calls: 1000,
            ..Limits::default()
        },
    )
    .expect("the vocabulary compiles");
    for order in support::permutations(&reactive) {
        let text = order.concat();
        assert_eq!(
            support::error_texts(&thousand, &text),
            inline_limited,
            "{text}"
        );
    }
}

#[test]
fn a_bounded_expansion_type_checks_every_branch_and_arm_while_it_expands_the_chosen_one() {
    let held = "input held: i32[..4] = [1, 2]\n";
    let read = "Clip c:\n    Note/(f(0))\n";
    let mismatched_arms = "fn f(i) bounded 3 else 0:\n    match i:\n        0: n + f(i + 1)\n        7: true\n        else: 1\n";
    let rows: &[support::Row] = &[
        (
            &[
                held,
                "fn f(i) bounded 3 else 0: if i > 5 then true else held[i] + f(i + 1)\n",
                read,
            ],
            &[(
                Code::TypeMismatch,
                "if i > 5 then true else held[i] + f(i + 1)",
            )],
        ),
        (
            &[
                held,
                "fn f(i) bounded 3 else 0: if i > 5 then 1.5 else held[i] + f(i + 1)\n",
                read,
            ],
            &[(
                Code::TypeMismatch,
                "if i > 5 then 1.5 else held[i] + f(i + 1)",
            )],
        ),
        (
            &[
                held,
                "fn g(i): if i > 5 then 1.5 else held[i] + 1\n",
                "Clip c:\n    Note/(g(0))\n",
            ],
            &[(Code::TypeMismatch, "if i > 5 then 1.5 else held[i] + 1")],
        ),
        (
            &["input n = 0\n", mismatched_arms, read],
            &[(
                Code::TypeMismatch,
                "match i:\n        0: n + f(i + 1)\n        7: true\n        else: 1\n",
            )],
        ),
        (
            &[
                held,
                "fn f(i) bounded 3 else 0: if i > 5 then [1, 2][7] else held[i] + f(i + 1)\n",
                read,
            ],
            &[],
        ),
        (
            &[
                held,
                "fn f(i) bounded 3 else 0: if i > 5 then true else f(i + 1) + 1\n",
                read,
            ],
            &[(Code::TypeMismatch, "if i > 5 then true else f(i + 1) + 1")],
        ),
        (
            &[
                held,
                "input note = 60\n",
                "fn f(i) bounded 3 else 0: if i > 5 then held else note + f(i + 1)\n",
                read,
            ],
            &[(
                Code::TypeMismatch,
                "if i > 5 then held else note + f(i + 1)",
            )],
        ),
        (
            &[
                held,
                "fn f(i) bounded 3 else [0]: if i > 5 then [[1]] else [held[i]] + f(i + 1)\n",
                "Clip c:\n    Note/(f(0)[0])\n",
            ],
            &[(
                Code::TypeMismatch,
                "if i > 5 then [[1]] else [held[i]] + f(i + 1)",
            )],
        ),
        (
            &[
                held,
                "fn f(i) bounded 3 else [0]: if i > 5 then [[1]] else [held[i]] + f(i + 1)\n",
                "let a = f(0)\n",
                "Clip c:\n    Note/(a[0])\n",
            ],
            &[(
                Code::TypeMismatch,
                "if i > 5 then [[1]] else [held[i]] + f(i + 1)",
            )],
        ),
        (
            &[
                held,
                "fn f(i) bounded 3 else [0]: if i > 5 then [1, 2, 3] else [held[i]] + f(i + 1)\n",
                "Clip c:\n    Note/(f(0)[0])\n",
            ],
            &[],
        ),
    ];
    support::assert_rows_in_every_order(&vocabulary(), rows);
}

fn a_call_rejected_as_reactive_recursion_adds_nothing_to_the_inline_count_in(
    orders: support::Orders,
) {
    let four = Compiler::with_limits(
        &vocabulary(),
        Limits {
            inlined_calls: 4,
            ..Limits::default()
        },
    )
    .expect("the vocabulary compiles");
    let h = "fn h(x): x + 1\n";
    let rows: &[support::Row] = &[
        (
            &[
                "input n = 0\n",
                h,
                "fn r(i): if i is 0 then 0 else r(i - 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(r(n))\n",
            ],
            &[(Code::ReactiveRecursion, "r(n)")],
        ),
        (
            &[
                "input x = 1\n",
                "let n = x + 2\n",
                h,
                "fn r(i): if i is 0 then 0 else r(i - 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(r(n))\n",
            ],
            &[(Code::ReactiveRecursion, "r(n)")],
        ),
        (
            &[
                "input x = 1\n",
                h,
                "fn r(i): if i < 1 then x else r(i - 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(r(3))\n",
            ],
            &[(Code::ReactiveRecursion, "r(3)")],
        ),
        (
            &[
                "input held = [1, 2, 3]\n",
                "fn r(i): if i is 0 then len(held) else r(i - 1)\n",
                "Clip c:\n    Note/(r(2))\n",
            ],
            &[],
        ),
        (
            &[h, "Clip c:\n    Note/(h(h(h(h(h(60))))))\n"],
            &[(Code::InlineLimit, "h(60)")],
        ),
        (&[h, "Clip c:\n    Note/(h(h(h(60))))\n"], &[]),
        (
            &[
                "input held: i32[..4] = []\n",
                h,
                "fn r(i): if i >= len(held) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "state held: i32[..4] = []\n",
                h,
                "fn r(i): if i >= len(held) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input note = 60\n",
                "state held = (held{1, []} + [note])[^4..]\n",
                h,
                "fn r(i): if i >= len(held) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = []\n",
                "fn r(i): if i >= len(held) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input fixed: i32[2] = [1, 2]\n",
                "fn r(i): if i >= len(fixed) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[],
        ),
        (
            &[
                "input fixed: i32[2] = [1, 2]\n",
                h,
                "fn r(i): if i >= len(fixed) then 0 else r(i + 1) + h(h(h(i)))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[],
        ),
        (
            &[
                "input note = 60\n",
                "state held = [note, note]\n",
                h,
                "fn r(i): if i >= len(held) then 0 else r(i + 1) + h(h(h(i)))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[],
        ),
        (
            &[
                "input held: i32[..4] = []\n",
                "input note = 60\n",
                "let grown = held + [note]\n",
                h,
                "fn r(i): if i >= len(grown) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = []\n",
                "input note = 60\n",
                "let grown: i32[..5] = held + [note]\n",
                h,
                "fn r(i): if i >= len(grown) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = []\n",
                "input note = 60\n",
                "let grown = held + [note]\n",
                h,
                "fn r(i, held = 0): if i >= len(grown) then held else r(i + 1, held) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input note = 60\n",
                "let one: i32[..5] = [note]\n",
                h,
                "fn r(i): if i >= len(one) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = []\n",
                "let tail = held[1..]\n",
                h,
                "fn r(i): if i >= len(tail) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = []\n",
                "let same = held\n",
                "let tail = same[1..]\n",
                h,
                "fn r(i): if i >= len(tail) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "let fixed = [1, 2]\n",
                "fn r(i): if i >= len(fixed) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[],
        ),
        (
            &[
                "input note = 60\n",
                "let cells = [note, note, note]\n",
                "fn r(i): if i >= len(cells) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[],
        ),
        (
            &[
                "input held: i32[..4] = []\n",
                "let cells = [len(held), 1]\n",
                "fn r(i): if i >= len(cells) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input note = 60\n",
                "input gate = false\n",
                "let pick = if gate then held else []\n",
                h,
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input note = 60\n",
                "input gate = false\n",
                "let pick = if gate then held else held[1..]\n",
                h,
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input note = 60\n",
                "input gate = false\n",
                "let pick = match note:\n    60: held\n    else: []\n",
                h,
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input note = 60\n",
                "input gate = false\n",
                "let pick = if false then held else held + [note]\n",
                h,
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input note = 60\n",
                "input gate = false\n",
                "let pick = if gate then [1] else [1, 2]\n",
                h,
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::ReactiveRecursion, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input gate = false\n",
                "let pick = if false then held else [1, 2]\n",
                h,
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::InlineLimit, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input gate = false\n",
                "let pick = if false then held else [1, 2]\n",
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input gate = false\n",
                "let pick = if gate then [1, 2] else [3, 4]\n",
                h,
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + h(h(h(h(h(i)))))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[(Code::InlineLimit, "r(0)")],
        ),
        (
            &[
                "input held: i32[..4] = [1, 2]\n",
                "input gate = false\n",
                "let pick = if gate then [1, 2] else [3, 4]\n",
                "fn r(i): if i >= len(pick) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            &[],
        ),
    ];
    for (items, expected) in rows {
        let mut expected: Vec<(Code, String)> = expected
            .iter()
            .map(|(code, text)| (*code, (*text).to_string()))
            .collect();
        expected.sort();
        for order in orders.of(items) {
            let text = order.concat();
            assert_eq!(support::error_texts(&four, &text), expected, "{text}");
        }
    }
}

#[test]
fn a_call_rejected_as_reactive_recursion_adds_nothing_to_the_inline_count() {
    a_call_rejected_as_reactive_recursion_adds_nothing_to_the_inline_count_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_call_rejected_as_reactive_recursion_adds_nothing_to_the_inline_count() {
    a_call_rejected_as_reactive_recursion_adds_nothing_to_the_inline_count_in(
        support::Orders::Every,
    );
}

fn the_recursion_depth_summary_names_each_function_of_its_cycle_once_in(orders: support::Orders) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let rows: &[(&[&str], &[&str])] = &[
        (
            &[
                "fn f(i): g(i)\n",
                "fn g(i): f(i + 1) + 1\n",
                "Clip c:\n    Note/(f(0))\n",
            ],
            &["f", "g"],
        ),
        (
            &["fn f(i): f(i + 1)\n", "Clip c:\n    Note/(f(0))\n"],
            &["f"],
        ),
        (
            &[
                "fn f(i): g(i)\n",
                "fn g(i): h(i)\n",
                "fn h(i): f(i + 1)\n",
                "Clip c:\n    Note/(f(0))\n",
            ],
            &["f", "g", "h"],
        ),
    ];
    for (items, names) in rows {
        for order in orders.of(items) {
            let text = order.concat();
            let found = support::errors(compiler.check(&[support::source(&text)]));
            let [diagnostic] = found.as_slice() else {
                panic!("{text}{found:?}");
            };
            assert_eq!(diagnostic.code, Code::RecursionDepth, "{text}");
            assert!(
                diagnostic.summary.lines().count() <= 3,
                "{text}{diagnostic:?}"
            );
            let (_, chain) = diagnostic.summary.rsplit_once(": ").unwrap_or_default();
            let mut named: Vec<&str> = chain.split(" → ").collect();
            named.sort_unstable();
            assert_eq!(named, *names, "{text}{diagnostic:?}");
        }
    }
    let bounded = [
        "fn f(i) bounded 500 else 0: g(i)\n",
        "fn g(i): f(i + 1) + 1\n",
        "Clip c:\n    Note/(60 + f(0))\n",
    ];
    for order in orders.of(&bounded) {
        let text = order.concat();
        let found = support::errors(compiler.check(&[support::source(&text)]));
        assert!(found.is_empty(), "{text}{found:?}");
    }
}

#[test]
fn the_recursion_depth_summary_names_each_function_of_its_cycle_once() {
    the_recursion_depth_summary_names_each_function_of_its_cycle_once_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_the_recursion_depth_summary_names_each_function_of_its_cycle_once() {
    the_recursion_depth_summary_names_each_function_of_its_cycle_once_in(support::Orders::Every);
}

#[test]
fn a_recursive_call_whose_array_argument_grows_finishes_within_the_limits() {
    array_arguments_finish_within_the_limits(&[3, 30], true);
}

#[test]
#[ignore = "stress"]
fn stress_a_recursive_call_whose_array_argument_grows_to_three_hundred_elements_finishes_within_the_limits()
 {
    array_arguments_finish_within_the_limits(&[300], false);
}

fn array_arguments_finish_within_the_limits(bounds: &[usize], finishing_rows: bool) {
    let mut rows: Vec<[String; 2]> = Vec::new();
    for bound in bounds {
        let grows = format!("fn f(xs): if len(xs) > {bound} then xs else f(xs + [1])\n");
        rows.push([grows.clone(), "Clip c:\n    Note/(f([1])[0])\n".into()]);
        rows.push([grows, "Clip c:\n    Note/(60 + len(f([1])))\n".into()]);
    }
    let finishing = [
        (
            "fn f(xs, n): if n > 3 then xs else f(xs + [1], n + 1)\n",
            "Clip c:\n    Note/(f([1], 0)[0])\n",
        ),
        (
            "fn f(xs, n): if n > 3 then xs else f([1] + xs, n + 1)\n",
            "Clip c:\n    Note/(f([1], 0)[0])\n",
        ),
        (
            "fn f(xs, n): if n > 3 then xs else f(xs + [[1]], n + 1)\n",
            "Clip c:\n    Note/(f([[1]], 0)[0][0])\n",
        ),
        (
            "fn f(xs) bounded 8 else xs: if len(xs) > 3 then xs else f(xs + [1])\n",
            "Clip c:\n    Note/(f([1])[0])\n",
        ),
        (
            "fn f(xs): if len(xs) > 0 then xs else f(xs + [1])\n",
            "Clip c:\n    Note/(f([1])[0])\n",
        ),
        (
            "fn f(xs, n): if n > 3 then xs else f(xs + [n], n + 1)\n",
            "Clip c:\n    Note/(f([], 0)[3])\n",
        ),
        (
            "fn f(xs): if len(xs) > 1 then f(xs[1..]) else xs\n",
            "Clip c:\n    Note/(f([1, 2, 3])[0])\n",
        ),
    ];
    for (function, clip) in finishing.into_iter().filter(|_| finishing_rows) {
        rows.push([function.into(), clip.into()]);
    }
    let items: Vec<[&str; 2]> = rows
        .iter()
        .map(|[function, clip]| [function.as_str(), clip.as_str()])
        .collect();
    let rows: Vec<support::Row> = items
        .iter()
        .map(|items| (items.as_slice(), [].as_slice()))
        .collect();
    support::assert_rows_in_every_order(&vocabulary(), &rows);
}

#[test]
#[ignore = "stress"]
fn stress_a_recursive_call_whose_array_argument_deepens_finishes_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(|| {
            let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
            for text in [
                "fn f(xs, n): if n > 3 then 0 else f([xs], n + 1)\nClip c:\n    Note/(f([1], 0))\n",
                "Clip c:\n    Note/(f([1], 0))\nfn f(xs, n): if n > 3 then 0 else f([xs], n + 1)\n",
            ] {
                let errors = support::errors(compiler.check(&[support::source(text)]));
                assert!(errors.len() <= 1, "{text}{errors:?}");
                for error in &errors {
                    assert_eq!(error.code, Code::RecursionDepth, "{text}{errors:?}");
                    assert!(error.summary.ends_with(": f"), "{text}{errors:?}");
                }
            }
        })
        .expect("the thread starts")
        .join()
        .expect("the check finishes without overflowing");
}
