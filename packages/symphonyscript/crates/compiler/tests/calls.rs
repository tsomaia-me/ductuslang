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

#[test]
fn a_call_that_a_bounded_expansion_repeats_is_named_once_with_its_levels() {
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
    let defaults = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    limited(&defaults, &format!("{input}{deep}{use_f}"), "999,999");
    let thousand = Compiler::with_limits(
        &vocabulary(),
        Limits {
            inlined_calls: 1000,
            ..Limits::default()
        },
    )
    .expect("the vocabulary compiles");
    for order in support::permutations(&[input, deep, use_f]) {
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
        for order in support::permutations(items) {
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
