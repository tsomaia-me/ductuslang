mod support;

use std::collections::BTreeSet;

use symphonyscript::{Code, Compiler, Diagnostic, Severity, Source};
use symphonyscript_core::Limits;
use symphonyscript_fixtures::vocabulary;

type Named = (Code, BTreeSet<String>);

fn named_errors(compiler: &Compiler, text: &str) -> Vec<Named> {
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let mut found: Vec<Named> = compiler
        .check(&[source])
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .map(|diagnostic| (diagnostic.code, chain_of(diagnostic)))
        .collect();
    found.sort();
    found
}

fn chain_of(diagnostic: &Diagnostic) -> BTreeSet<String> {
    if !matches!(
        diagnostic.code,
        Code::DeclarationCycle | Code::RecursionDepth
    ) {
        return BTreeSet::new();
    }
    let chain = diagnostic
        .summary
        .rsplit_once(": ")
        .map_or("", |(_, chain)| chain);
    chain.split(" → ").map(str::to_string).collect()
}

fn named(code: Code, names: &[&str]) -> Named {
    (code, names.iter().map(|name| (*name).to_string()).collect())
}

fn first_order_failing(compiler: &Compiler, items: &[&str], wanted: &[Named]) -> Option<String> {
    support::permutations(items).into_iter().find_map(|order| {
        let text = format!("{}\n", order.join("\n"));
        let found = named_errors(compiler, &text);
        (found != wanted).then(|| format!("{text}expected {wanted:?}\nfound {found:?}"))
    })
}

#[test]
fn a_cycle_in_the_demand_engine_draws_its_one_rules_diagnostic() {
    let k_and_g = ["let k = g(1)", "fn g(n): if n < 1 then k else g(n - 1)"];
    let through = |reader: &'static str| -> Vec<&'static str> {
        k_and_g.iter().copied().chain([reader]).collect()
    };
    let mutual = [
        "fn a(n): if n < 1 then b(1) else a(n - 1)",
        "fn b(n): if n < 1 then a(1) else b(n - 1)",
        "Clip c:\n    Note/(a(1))",
    ];
    let endless =
        |function: &'static str, use_line: &'static str| -> (Vec<&'static str>, Vec<Named>) {
            (
                vec![function, use_line],
                vec![(
                    Code::RecursionDepth,
                    BTreeSet::from([function[3..4].to_string()]),
                )],
            )
        };
    let rows: Vec<(Vec<&str>, Vec<Named>)> = vec![
        endless("fn f(): f()", "Clip c:\n    Note/(f())"),
        endless("fn f(n = 1): f()", "Clip c:\n    Note/(f())"),
        endless("fn h(): 1 + h()", "Clip c:\n    Note/(h())"),
        endless("fn k(): if true then k() else 0", "Clip c:\n    Note/(k())"),
        endless("fn f(n): f(n + 1)", "Clip c:\n    Note/(f(0))"),
        endless("fn f(n): f(n + 1) + 1", "Clip c:\n    Note/(f(0))"),
        endless("fn f(n): n + f(n - 1)", "Clip c:\n    Note/(f(3))"),
        endless(
            "fn f(n): if n < 1 then 0 else f(n + 1)",
            "Clip c:\n    Note/(f(1))",
        ),
        (
            vec!["fn f(): f()", "let x = f()", "Clip c:\n    Note/60"],
            vec![],
        ),
        endless("fn f(x = f()): x", "Clip c:\n    Note/(f())"),
        (
            vec!["fn f(x = g()): x", "fn g(): f()", "Clip c:\n    Note/(f())"],
            vec![named(Code::RecursionDepth, &["f", "g"])],
        ),
        (
            vec![
                "fn f(x = g()) bounded 3 else 0: x",
                "fn g(): f()",
                "Clip c:\n    Note/(f() + 60)",
            ],
            vec![named(Code::RecursionDepth, &["f", "g"])],
        ),
        endless(
            "fn f(n, x = f(0)): if n < 1 then 7 else x",
            "Clip c:\n    Note/(f(3))",
        ),
        (
            vec![
                "fn f(n, x = f(0, 1)): if n < 1 then x else n",
                "Clip c:\n    Note/(f(3))",
            ],
            vec![],
        ),
        (
            vec![
                "fn f(n): if n < 1 then f(0) else 1",
                "Clip c:\n    Note/(f(1))\n    Note/(f(0))",
            ],
            vec![named(Code::RecursionDepth, &["f"])],
        ),
        (
            mutual.to_vec(),
            vec![named(Code::RecursionDepth, &["a", "b"])],
        ),
        (
            through("Clip c:\n    Note/k"),
            vec![named(Code::DeclarationCycle, &["k", "g"])],
        ),
        (
            through("Clip c:\n    Note/(g(2))"),
            vec![named(Code::DeclarationCycle, &["k", "g"])],
        ),
        (
            through("Clip c:\n    Note/60"),
            vec![named(Code::DeclarationCycle, &["k", "g"])],
        ),
        (
            vec![
                "fn g(n): if n < 1 then c.gain else g(n - 1)",
                "Clip c | gain=(g(1))",
            ],
            vec![named(Code::DeclarationCycle, &["c.gain", "g"])],
        ),
        (
            vec![
                "fn f(n): if n < 1 then a else f(n - 1)",
                "let a = f(3)",
                "Clip c:\n    Note/a",
            ],
            vec![named(Code::DeclarationCycle, &["a", "f"])],
        ),
        (
            vec![
                "fn f(n): if n < 1 then a else f(n - 1)",
                "let a = c.gain",
                "Clip c | gain=(f(3))",
            ],
            vec![named(Code::DeclarationCycle, &["a", "c.gain", "f"])],
        ),
        (
            vec![
                "fn f(n, m = a): if n < 1 then m else f(n - 1)",
                "let a = f(3)",
                "Clip c:\n    Note/a",
            ],
            vec![named(Code::DeclarationCycle, &["a", "f", "m"])],
        ),
        (
            vec![
                "let x = [1, 2, 3][i32(d(2))]",
                "fn d(n, z = r(2)): if n < 1 then z else d(n - 1)",
                "fn r(n): if n < 1 then x else r(n - 1)",
            ],
            vec![named(Code::DeclarationCycle, &["x", "d", "z", "r"])],
        ),
        (
            vec![
                "Clip d | gain=c.w.gain:\n    Note",
                "Clip c:\n    Plays as w | gain=to.gain: d",
            ],
            vec![named(Code::DeclarationCycle, &["d.gain", "w.gain"])],
        ),
        (
            vec!["Clip c:\n    Plays as w | gain=to.gain: w"],
            vec![named(Code::DestinationType, &[])],
        ),
        (
            vec!["Clip c:\n    Plays as w: w"],
            vec![named(Code::DestinationType, &[])],
        ),
        (
            vec![
                "input q = 0",
                "Clip c:\n    Plays as w | gain=to.gain: [w, c][q]",
            ],
            vec![named(Code::DestinationType, &[])],
        ),
    ];
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let mut failures: Vec<String> = rows
        .iter()
        .filter_map(|(items, wanted)| first_order_failing(&compiler, items, wanted))
        .collect();
    let shallow = Limits {
        nesting_depth: 6,
        ..Limits::default()
    };
    let compiler = Compiler::with_limits(&vocabulary(), shallow).expect("the vocabulary compiles");
    failures.extend(first_order_failing(
        &compiler,
        &mutual,
        &[named(Code::RecursionDepth, &["a", "b"])],
    ));
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

const LINKS: [(&[&str], &str); 8] = [
    (&["let l{k} = {next}"], "l{k}"),
    (
        &["fn f{k}(n): if n < 1 then {next} else f{k}(n - 1)"],
        "f{k}(1)",
    ),
    (
        &["fn d{k}(n, z = {next}): if n < 1 then z else d{k}(n - 1)"],
        "d{k}(1)",
    ),
    (&["Clip i{k} | gain=({next})"], "i{k}.gain"),
    (
        &["Clip s{k}:\n    Plays as w{k} | gain=({next}): s{k}"],
        "s{k}.w{k}.gain",
    ),
    (
        &["Clip o{k} | gain=({next}):\n    Plays as w{k} | gain=to.gain: o{k}"],
        "o{k}.w{k}.gain",
    ),
    (
        &[
            "input q{k} = 0",
            "Clip o{k} | gain=({next}):\n    Plays as w{k} | gain=to.gain: [o{k}, o{k}][q{k}]",
        ],
        "o{k}.w{k}.gain",
    ),
    (
        &[
            "pattern P{k}(x):\n    Clip | gain=x",
            "P{k} u{k} | x=({next})",
        ],
        "u{k}.gain",
    ),
];

const FUNCTION_LINKS: [usize; 2] = [1, 2];

fn cycle_items(links: &[usize], closing: Option<&str>) -> Vec<String> {
    let mut items = Vec::new();
    for (position, link) in links.iter().enumerate() {
        let after = (position + 1) % links.len();
        let next = match closing {
            Some(literal) if after == 0 => literal.to_string(),
            _ => LINKS[links[after]].1.replace("{k}", &after.to_string()),
        };
        for line in LINKS[*link].0 {
            items.push(
                line.replace("{k}", &position.to_string())
                    .replace("{next}", &next),
            );
        }
    }
    items
}

fn cycles_up_to(length: u32) -> Vec<Vec<usize>> {
    let mut cycles = Vec::new();
    for links in 1..=length {
        for code in 0..LINKS.len().pow(links) {
            let cycle: Vec<usize> = (0..links)
                .map(|position| code / LINKS.len().pow(position) % LINKS.len())
                .collect();
            let first_rotation = (1..cycle.len()).all(|turn| {
                let rotated: Vec<usize> = cycle[turn..]
                    .iter()
                    .chain(&cycle[..turn])
                    .copied()
                    .collect();
                rotated >= cycle
            });
            let declares = cycle.iter().any(|link| !FUNCTION_LINKS.contains(link));
            if first_rotation && declares {
                cycles.push(cycle);
            }
        }
    }
    cycles
}

fn every_cycle_of_declarations_draws_one_declaration_cycle_in_every_order_in(
    orders: support::Orders,
) {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let cycles = cycles_up_to(3);
    let failures: Vec<String> = std::thread::scope(|threads| {
        let runs: Vec<_> = cycles
            .chunks(cycles.len().div_ceil(8))
            .map(|chunk| {
                let compiler = &compiler;
                threads.spawn(move || {
                    let mut failures = Vec::new();
                    for cycle in chunk {
                        for (closing, wanted) in
                            [(None, vec![Code::DeclarationCycle]), (Some("1"), vec![])]
                        {
                            let items = cycle_items(cycle, closing);
                            let lines: Vec<&str> = items.iter().map(String::as_str).collect();
                            let failure = orders.of(&lines).into_iter().find_map(|order| {
                                let text = format!("{}\n", order.join("\n"));
                                let found: Vec<Code> = named_errors(compiler, &text)
                                    .into_iter()
                                    .map(|(code, _)| code)
                                    .collect();
                                (found != wanted).then(|| format!("{text}found {found:?}"))
                            });
                            failures.extend(failure);
                        }
                    }
                    failures
                })
            })
            .collect();
        runs.into_iter()
            .flat_map(|run| run.join().expect("every run finishes"))
            .collect()
    });
    assert_eq!(cycles.len(), 211);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn every_cycle_of_declarations_draws_one_declaration_cycle_forward_and_reversed() {
    every_cycle_of_declarations_draws_one_declaration_cycle_in_every_order_in(
        support::Orders::ForwardAndReversed,
    );
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_every_cycle_of_declarations_draws_one_declaration_cycle_in_every_order() {
    every_cycle_of_declarations_draws_one_declaration_cycle_in_every_order_in(
        support::Orders::Every,
    );
}

fn secondary_spans(compiler: &Compiler, text: &str) -> Vec<Vec<(u32, u32)>> {
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    compiler
        .check(&[source])
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::RecursionDepth)
        .map(|diagnostic| {
            diagnostic
                .related
                .iter()
                .map(|related| &related.span)
                .map(|span| (span.range.start, span.range.end))
                .collect()
        })
        .collect()
}

#[test]
fn an_evaluation_that_goes_too_deep_names_the_outermost_call_it_ends() {
    let rows: [(&[&str], &str, &str, Limits); 4] = [
        (
            &[
                "fn f(n): if n < 1 then f(0) else 1",
                "Clip c:\n    Note/(f(1))\n    Note/(f(0))",
            ],
            "Note/(f(0))",
            "f(0)",
            Limits::default(),
        ),
        (
            &[
                "fn a(n): if n < 1 then b(1) else a(n - 1)",
                "fn b(n): if n < 1 then a(1) else b(n - 1)",
                "Clip c:\n    Note/(a(1))",
            ],
            "Note/(a(1))",
            "a(1)",
            Limits::default(),
        ),
        (
            &[
                "fn down(n): if n < 1 then 0 else down(n - 1)",
                "Clip c:\n    Note/(down(1024))",
            ],
            "Note/(down(1024))",
            "down(1024)",
            Limits::default(),
        ),
        (
            &[
                "fn even(n): if n < 1 then true else odd(n - 1)",
                "fn odd(n): if n < 1 then false else even(n - 1)",
                "Clip c:\n    Note/(if even(10) then 1 else 2)",
            ],
            "Note/(if even(10) then 1 else 2)",
            "even(10)",
            Limits {
                nesting_depth: 6,
                ..Limits::default()
            },
        ),
    ];
    let mut failures = Vec::new();
    for (items, use_line, call, limits) in rows {
        let compiler =
            Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
        for order in support::permutations(items) {
            let text = format!("{}\n", order.join("\n"));
            let (line, _) = support::at(&text, use_line, 0);
            let offset = use_line.find(call).and_then(|at| u32::try_from(at).ok());
            let length = u32::try_from(call.len()).ok();
            let wanted: Vec<Vec<(u32, u32)>> = offset
                .zip(length)
                .map(|(offset, length)| vec![(line + offset, line + offset + length)])
                .into_iter()
                .collect();
            let found = secondary_spans(&compiler, &text);
            if found != wanted {
                failures.push(format!("{text}expected {wanted:?}\nfound {found:?}"));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn wire_cycles_draw_one_diagnostic_per_component_naming_its_shortest_cycle_and_wires() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let two_cycle =
        "Synth s:\n    Filter as a:\n        Feeds: b\n    Filter as b:\n        Feeds: a\n";
    let other_two_cycle =
        "Synth t:\n    Filter as d:\n        Feeds: e\n    Filter as e:\n        Feeds: d\n";
    type Row<'a> = (&'a [&'a str], &'a [(&'a str, &'a str)]);
    let rows: [Row; 7] = [
        (
            &[
                "Synth s:\n    Filter as a:\n        Feeds: b\n        Feeds: c\n    Filter as b:\n        Feeds: a\n    Filter as c:\n        Feeds: a\n",
            ],
            &[("a → b → a", "4 wires")],
        ),
        (&[two_cycle], &[("a → b → a", "2 wires")]),
        (
            &[two_cycle, other_two_cycle],
            &[("a → b → a", "2 wires"), ("d → e → d", "2 wires")],
        ),
        (
            &["Synth s:\n    Filter as a:\n        Feeds: a\n"],
            &[("a → a", "1 wire")],
        ),
        (
            &[
                "input i = 0\n",
                "Synth s:\n    Filter as a:\n        Feeds: [b, c][i]\n    Filter as b\n    Filter as c:\n        Feeds: a\n",
            ],
            &[("a → c → a", "2 wires")],
        ),
        (
            &[
                "Clip a:\n    Note\n    Plays: b\n",
                "Clip b:\n    Note\n    Plays: a\n",
            ],
            &[],
        ),
        (
            &[
                "Synth s:\n    Osc as o:\n        Feeds: f\n    Filter as f:\n        Feeds: m\n    Mixer as m\n",
            ],
            &[],
        ),
    ];
    for (items, wanted) in rows {
        let wanted: Vec<String> = wanted
            .iter()
            .map(|(cycle, wires)| {
                format!("closes the cycle of wires {cycle}, and its group of cycles holds {wires};")
            })
            .collect();
        for order in support::permutations(items) {
            let text = order.concat();
            let errors = support::errors(compiler.check(&[support::source(&text)]));
            let mut found: Vec<String> = errors
                .iter()
                .map(|diagnostic| {
                    let held = diagnostic
                        .summary
                        .split_once("closes")
                        .map_or("", |(_, rest)| rest);
                    format!("{:?} closes{held}", diagnostic.code)
                })
                .collect();
            found.sort();
            let mut expected: Vec<String> = wanted
                .iter()
                .map(|fragment| format!("WireCycle {fragment}"))
                .collect();
            expected.sort();
            let matches = found.len() == expected.len()
                && found
                    .iter()
                    .zip(&expected)
                    .all(|(found, expected)| found.starts_with(expected.as_str()));
            assert!(matches, "{text}expected {expected:?}\nfound {found:?}");
        }
    }
}

#[test]
fn a_declaration_cycle_drawn_before_the_recursive_call_limit_stops_the_compilation_is_kept() {
    let limits = Limits {
        recursive_calls: 5,
        ..Limits::default()
    };
    let compiler = Compiler::with_limits(&vocabulary(), limits).expect("the vocabulary compiles");
    let text =
        "let a = b\nlet b = a\nfn r(n): if n < 1 then 0 else r(n - 1)\nClip c:\n    Note/(r(10))\n";
    assert_eq!(
        named_errors(&compiler, text),
        vec![
            named(Code::DeclarationCycle, &["a", "b"]),
            named(Code::RecursionLimit, &[])
        ]
    );
}
