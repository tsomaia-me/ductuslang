mod support;

use symphonyscript::{Code, Diagnostic};

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
