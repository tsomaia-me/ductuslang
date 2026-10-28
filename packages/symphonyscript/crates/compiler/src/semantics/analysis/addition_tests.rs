use symphonyscript_core::{BinaryOp, Limits, Scalar};
use symphonyscript_fixtures::{native_vocabulary, vocabulary};

use super::Engine;
use super::attr_value_source::AttrValueSource;
use super::bound::Bound;
use super::bound_val::BoundVal;
use super::c_array::CArray;
use super::c_val::CVal;
use super::eval_ty::EvalTy;
use super::ids::{BindingId, NodeId};
use super::op::Op;
use super::val::Val;
use crate::compiler::Compiler;
use crate::diagnostics::{Diagnostic, Severity};
use crate::semantics::ir::StateId;
use crate::source::Source;

fn analyzed<R>(
    vocabulary: &symphonyscript_core::Vocabulary,
    text: &str,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> (Vec<Diagnostic>, R) {
    analyzed_within(vocabulary, Limits::default(), text, inspect)
}

fn analyzed_within<R>(
    vocabulary: &symphonyscript_core::Vocabulary,
    limits: Limits,
    text: &str,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> (Vec<Diagnostic>, R) {
    let compiler = Compiler::with_limits(vocabulary, limits).expect("the vocabulary compiles");
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let (diagnostics, found) = compiler.analyzed(&[source], inspect);
    (diagnostics, found.expect("the run meets no internal error"))
}

fn errors(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .collect()
}

fn state_capacity(engine: &Engine<'_>, state: u32) -> Option<u32> {
    let Some(BoundVal::Scalar(Bound::Cell(binding))) = engine.memo.states.get(&StateId(state))
    else {
        return None;
    };
    engine.memo.binding_capacities.get(binding).copied()
}

fn recent(bound: u64) -> String {
    format!(
        "input note = 60\nstate recent = (recent{{1, []}} + [note])[^{bound}..]\nClip c:\n    Note/(recent[^1])\n"
    )
}

#[test]
fn a_state_bounded_by_a_slice_finds_its_capacity_in_work_that_does_not_depend_on_the_bound() {
    let mut found = Vec::new();
    for bound in [8u64, 1_000, 1_000_000_000] {
        let (diagnostics, (capacity, rounds)) = analyzed(&vocabulary(), &recent(bound), |engine| {
            (state_capacity(engine, 0), engine.memo.capacity_rounds)
        });
        assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
        assert_eq!(capacity, u32::try_from(bound).ok());
        found.push(rounds);
    }
    assert!(found.windows(2).all(|pair| pair[0] == pair[1]), "{found:?}");
}

fn ring_rounds(states: u32) -> u64 {
    let mut text = String::from("input note = 60\n");
    for state in 0..states {
        let previous = (state + states - 1) % states;
        text.push_str(&format!(
            "state s{state} = (s{previous}{{1, []}} + [note])[^8..]\n"
        ));
    }
    text.push_str("Clip c:\n    Note/(s0[0])\n");
    let (diagnostics, (capacities, rounds)) = analyzed(&vocabulary(), &text, |engine| {
        let capacities: Vec<Option<u32>> = (0..states)
            .map(|state| state_capacity(engine, state))
            .collect();
        (capacities, engine.memo.capacity_rounds)
    });
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert!(
        capacities.iter().all(|capacity| *capacity == Some(8)),
        "{capacities:?}"
    );
    rounds
}

#[test]
fn a_ring_of_states_takes_its_capacities_in_work_linear_in_it() {
    let (small, large) = (ring_rounds(250), ring_rounds(1_000));
    assert!(large <= 4 * small, "{small} {large}");
}

fn settlement_visits(states: u32, reversed: bool, read: impl Fn(u32) -> u32) -> u64 {
    let mut lines: Vec<String> = (0..states)
        .map(|state| {
            let target = read(state);
            format!("state s{state} = (s{target}{{1, []}} + [note])[^8..]\n")
        })
        .collect();
    if reversed {
        lines.reverse();
    }
    let text = format!(
        "input note = 60\n{}Clip c:\n    Note/(s0[0])\n",
        lines.concat()
    );
    let (diagnostics, visits) =
        analyzed(&vocabulary(), &text, |engine| engine.memo.settlement_visits);
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    visits
}

#[test]
fn settling_a_group_of_states_costs_work_linear_in_it_in_every_declaration_order() {
    type Shape = fn(u32, u32) -> u32;
    let shapes: [(&str, Shape); 3] = [
        ("ring", |state, states| (state + states - 1) % states),
        ("chain", |state, _| state.saturating_sub(1)),
        ("independent", |state, _| state),
    ];
    for (name, shape) in shapes {
        for reversed in [false, true] {
            let small = settlement_visits(250, reversed, |state| shape(state, 250));
            let large = settlement_visits(1_000, reversed, |state| shape(state, 1_000));
            assert!(
                large <= 4 * small + 64,
                "{name} reversed {reversed}: {small} {large}"
            );
        }
    }
}

fn kind_walk(text: &str, expected: &[crate::diagnostics::Code]) -> (u64, u64) {
    let (diagnostics, visits) = analyzed(&vocabulary(), text, |engine| {
        (engine.memo.kind_visits, engine.memo.settlement_visits)
    });
    let codes: Vec<crate::diagnostics::Code> = errors(&diagnostics)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, expected, "{diagnostics:?}");
    visits
}

fn kind_walk_program(shape: &str, states: u32) -> String {
    let last = states - 1;
    let lines: Vec<String> = (0..states)
        .map(|state| {
            let previous = (state + last) % states;
            let next = (state + 1) % states;
            match shape {
                "forward ring" | "reversed ring" => {
                    format!("state s{state} = (s{previous}{{1, []}} + [note])[^8..]\n")
                }
                "integer chain" if state == 0 => String::from("state p0 = y{1, 0}\n"),
                "integer chain" => format!("state p{state} = p{previous}{{1, 0}}\n"),
                "float chain" if state == 0 => String::from("state p0 = y{1, 0.0}\n"),
                "float chain" => format!("state p{state} = p{previous}{{1, 0.0}}\n"),
                "float at the end" if state == last => {
                    format!("state p{state} = p0{{1, 0.5}}\n")
                }
                "float at the end" => format!("state p{state} = p{next}{{1, 0}}\n"),
                "float at the start" if state == 0 => {
                    format!("state p0 = p{last}{{1, 0.5}}\n")
                }
                "float at the start" => format!("state p{state} = p{previous}{{1, 0}}\n"),
                _ => format!("state p{state} = y{{1, 0}}\n"),
            }
        })
        .collect();
    let body = if shape == "reversed ring" {
        lines.iter().rev().cloned().collect::<String>()
    } else {
        lines.concat()
    };
    let (head, read) = match shape {
        "forward ring" | "reversed ring" => (
            "input note = 60\n",
            format!("Clip c:\n    Note/(s{last}[0])\n"),
        ),
        "integer chain" => (
            "input x = 1\nlet y = x * 2\n",
            format!("Clip c:\n    Note/(p{last})\n"),
        ),
        "float chain" => (
            "input x = 1\nlet y = x * 0.5\n",
            format!("Clip c:\n    Note/(p{last})\n"),
        ),
        "float at the end" | "float at the start" => {
            ("", String::from("Clip c:\n    Note | brightness=p0\n"))
        }
        _ => (
            "input x = 1.5\nlet y = x * 2\n",
            format!("Clip c:\n    Note | brightness=p{last}\n"),
        ),
    };
    format!("{head}{body}{read}")
}

#[test]
fn the_kind_walk_over_a_group_of_states_is_linear_in_every_declaration_order() {
    use crate::diagnostics::Code;
    let shapes: [(&str, &[Code]); 7] = [
        ("forward ring", &[]),
        ("reversed ring", &[]),
        ("integer chain", &[]),
        ("float chain", &[Code::FloatToInteger]),
        ("float at the end", &[]),
        ("float at the start", &[]),
        ("fan", &[]),
    ];
    for (shape, expected) in shapes {
        let visits: Vec<(u64, u64)> = [2_500, 5_000, 10_000]
            .into_iter()
            .map(|states| kind_walk(&kind_walk_program(shape, states), expected))
            .collect();
        for pair in visits.windows(2) {
            let (smaller, larger) = (pair[0], pair[1]);
            assert!(larger.0 <= 2 * smaller.0 + 64, "{shape}: {visits:?}");
            assert!(larger.1 <= 2 * smaller.1 + 64, "{shape}: {visits:?}");
        }
    }
}

fn wire_cycle_work(filters: u32, feeds: impl Fn(u32) -> Vec<u32>) -> (usize, u64) {
    let body: String = (0..filters)
        .map(|filter| {
            let wires: String = feeds(filter)
                .iter()
                .map(|target| format!("        Feeds: node{target}\n"))
                .collect();
            format!("    Filter as node{filter}:\n{wires}")
        })
        .collect();
    let text = format!("Synth s:\n{body}");
    let (diagnostics, work) = analyzed(&vocabulary(), &text, |engine| engine.memo.cycle_work);
    (errors(&diagnostics).len(), work)
}

#[test]
fn wire_cycles_are_found_in_work_linear_in_the_instances_and_wires() {
    type Feeds = fn(u32, u32) -> Vec<u32>;
    type Cycles = fn(u32) -> usize;
    let shapes: [(&str, Feeds, Cycles); 2] = [
        (
            "ring",
            |filter, filters| vec![(filter + 1) % filters],
            |_| 1,
        ),
        (
            "chain of two-cycles",
            |filter, filters| {
                let partner = filter ^ 1;
                let next = filter + 2;
                if filter % 2 == 0 && next < filters {
                    vec![partner, next]
                } else {
                    vec![partner]
                }
            },
            |filters| usize::try_from(filters / 2).unwrap_or(usize::MAX),
        ),
    ];
    for (name, shape, cycles) in shapes {
        let (small_cycles, small) = wire_cycle_work(250, |filter| shape(filter, 250));
        let (large_cycles, large) = wire_cycle_work(1_000, |filter| shape(filter, 1_000));
        assert_eq!(
            (small_cycles, large_cycles),
            (cycles(250), cycles(1_000)),
            "{name}"
        );
        assert!(large <= 4 * small + 64, "{name}: {small} {large}");
    }
}

#[test]
fn states_whose_history_reads_reach_one_another_take_their_least_capacities_together() {
    let text = "input note = 60\nstate p = (q{1, []} + [note])[^3..]\nstate q = p{1, []} + [1]\nClip c:\n    Note/(p[0] + q[0])\n";
    let (diagnostics, capacities) = analyzed(&vocabulary(), text, |engine| {
        (state_capacity(engine, 0), state_capacity(engine, 1))
    });
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert_eq!(capacities, (Some(3), Some(4)));
}

fn expansion_size(levels: usize) -> usize {
    let text = format!(
        "input held: i32[..4] = []\nfn walk(i) bounded {levels} else 0:\n    if i >= len(held) then 0 else held[i] + walk(i + 1)\nClip c:\n    Note/(walk(0))\n"
    );
    let (diagnostics, nodes) = analyzed(&vocabulary(), &text, |engine| engine.nodes.len());
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    nodes
}

#[test]
fn a_bounded_call_expands_in_work_linear_in_its_levels() {
    let (small, middle, large) = (
        expansion_size(100),
        expansion_size(200),
        expansion_size(400),
    );
    assert_eq!(large - middle, 2 * (middle - small));
}

#[test]
fn a_state_settled_as_a_fixed_array_expands_a_loop_over_its_length() {
    let items = [
        "input note = 60\n",
        "state s = (s{1, [0, 0]} + [note])[1..]\n",
        "Clip c:\n    for i in 0..len(s):\n        Note/(s[i])\n",
    ];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let text: String = order.iter().map(|index| items[*index]).collect();
        let (diagnostics, notes) = analyzed(&vocabulary(), &text, |engine| {
            engine
                .instances
                .iter()
                .filter(|instance| {
                    engine
                        .vocabulary
                        .node_kind(instance.kind)
                        .is_some_and(|decl| decl.name == "Note")
                })
                .count()
        });
        assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
        assert_eq!(notes, 2, "{text}");
    }
}

#[test]
fn a_for_expands_once_per_element_or_per_value_of_its_computed_range() {
    let q = "fn q(n): if n < 1 then 0 else q(n - 1)\n";
    let rows = [
        (
            String::from(
                "fn g(x): x + 1\nClip c:\n    for i in [1, 2]:\n        Note\n    for i in [g(3)]:\n        Note\n",
            ),
            3,
        ),
        (
            String::from("let n = 3\nClip c:\n    for t in 0..n:\n        Note\n"),
            3,
        ),
        (
            String::from(
                "fn f(n): if n < 1 then 2 else f(n - 1)\nClip c:\n    for i in 0..f(3):\n        Note\n",
            ),
            2,
        ),
        (
            String::from("fn z(): 2\nClip c:\n    for i in 0..z():\n        Note\n"),
            2,
        ),
        (
            format!("{q}Clip c:\n    for i in q(2)..2:\n        Note\n"),
            2,
        ),
        (format!("{q}Clip c:\n    Note/([5, 6, 7][q(2)])\n"), 1),
    ];
    for (text, expected) in rows {
        let (diagnostics, (notes, pitch)) = analyzed(&vocabulary(), &text, |engine| {
            let notes = engine
                .instances
                .iter()
                .filter(|instance| {
                    engine
                        .vocabulary
                        .node_kind(instance.kind)
                        .is_some_and(|decl| decl.name == "Note")
                })
                .count();
            (notes, attr_value(engine, "pitch", EvalTy::I32))
        });
        assert!(errors(&diagnostics).is_empty(), "{text}: {diagnostics:?}");
        assert_eq!(notes, expected, "{text}");
        if text.contains("[5, 6, 7]") {
            assert_eq!(pitch, Some(CVal::Int(5)), "{text}");
        }
    }
}

#[test]
fn a_recursive_call_evaluates_its_omitted_default_and_yields_its_body() {
    let rows: [(&str, &str, i128); 4] = [
        (
            "fn f(n, x = f(0, 1)): if n < 1 then x else n\n",
            "Note/(f(3))",
            3,
        ),
        (
            "fn f(x, y = f(x)) bounded 3 else 0: x\n",
            "Note/(f(1, 0) + 60)",
            61,
        ),
        (
            "fn f(x, y) bounded 3 else 0: f(x, y)\n",
            "Note/(f(1, 0) + 60)",
            60,
        ),
        (
            "fn f(x, y = x + 1) bounded 3 else 0: if x > 0 then f(x - 1) + y else y\n",
            "Note/(f(2) + 60)",
            66,
        ),
    ];
    for (function, read, expected) in rows {
        let clip = format!("Clip c:\n    {read}\n");
        for text in in_every_order(&[function, &clip]) {
            let (diagnostics, pitch) = analyzed(&vocabulary(), &text, |engine| {
                attr_value(engine, "pitch", EvalTy::I32)
            });
            assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
            assert_eq!(pitch, Some(CVal::Int(expected)), "{text}");
        }
    }
}

fn attr_value(engine: &Engine<'_>, name: &str, ty: EvalTy) -> Option<CVal> {
    let instance = engine.instances.last()?;
    let decl = engine.vocabulary.node_kind(instance.kind)?;
    let source = instance.attrs.iter().find(|source| {
        decl.attrs
            .get(source.attr.index())
            .is_some_and(|attr| attr.name == name)
    })?;
    let AttrValueSource::Expr(expr, scope) = source.value else {
        return None;
    };
    let Some(Val::Scalar(node)) = engine.memo.exprs.get(&(expr, scope)) else {
        return None;
    };
    engine.memo.evals.get(&(*node, ty)).cloned()
}

#[test]
fn a_native_function_is_called_at_compile_time_with_compile_time_arguments() {
    let cases: [(&str, i128); 8] = [
        ("checksum(\"GET\")", 71 + 69 + 84),
        ("checksum([u8(1), u8(2), u8(3)])", 6),
        ("text_length(\"hello\")", 5),
        ("text_length(\"abc\")", 3),
        ("pair_sum([1, 2])", 3),
        ("first_of([1, 2, 3, 4])", 1),
        ("mirror([5, 6, 7])[0]", 7),
        ("len(mirror([5, 6, 7]))", 3),
    ];
    for (call, expected) in cases {
        let text = format!("Clip c:\n    Note/({call})\n");
        let (diagnostics, value) = analyzed(&native_vocabulary(), &text, |engine| {
            attr_value(engine, "pitch", EvalTy::I32)
        });
        assert!(errors(&diagnostics).is_empty(), "{call}: {diagnostics:?}");
        assert_eq!(value, Some(CVal::Int(expected)), "{call}");
    }
}

#[test]
fn bytes_converts_each_integer_as_u8_converts() {
    let cases: [(&str, i128); 4] = [
        ("packed[0]", 255),
        ("packed[1]", 0),
        ("packed[2]", 255),
        ("packed[3] + len(packed)", 16),
    ];
    for (read, expected) in cases {
        let text =
            format!("let packed = bytes([300, -5, 70000, 12])\nClip c:\n    Note/({read})\n");
        let (diagnostics, value) = analyzed(&vocabulary(), &text, |engine| {
            attr_value(engine, "pitch", EvalTy::I32)
        });
        assert!(errors(&diagnostics).is_empty(), "{read}: {diagnostics:?}");
        assert_eq!(value, Some(CVal::Int(expected)), "{read}");
    }
}

#[test]
fn a_bounded_call_yields_its_fallback_nested_inside_its_bound() {
    let cases: [(&str, i128); 3] = [
        ("fn count(i) bounded 3 else 100: count(i + 1) + 1\n", 103),
        ("fn count(i) bounded 2 else 100: count(i) + 1\n", 102),
        (
            "fn count(i) bounded 2 else 0: g(i) + 1\nfn g(i) bounded 2 else 0: count(i) + 10\n",
            22,
        ),
    ];
    for (functions, expected) in cases {
        let text = format!("{functions}Clip c:\n    Note/(count(0))\n");
        let (diagnostics, value) = analyzed(&vocabulary(), &text, |engine| {
            attr_value(engine, "pitch", EvalTy::I32)
        });
        assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
        assert_eq!(value, Some(CVal::Int(expected)), "{functions}");
    }
}

#[test]
fn a_fixed_u8_argument_computes_in_u8_through_a_recursive_body() {
    let text =
        "fn r(x, n): if n < 1 then x + u8(1) else r(x, n - 1)\nClip c:\n    Note/(r(u8(3), 1))\n";
    let (diagnostics, value) = analyzed(&vocabulary(), text, |engine| {
        attr_value(engine, "pitch", EvalTy::I32)
    });
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert_eq!(value, Some(CVal::Int(4)));
}

fn deepening_work(nesting_depth: u64) -> u64 {
    let limits = Limits {
        nesting_depth,
        ..Limits::default()
    };
    let text =
        "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - -1)\nClip c:\n    Note/(fib(6))\n";
    let (diagnostics, work) = analyzed_within(&vocabulary(), limits, text, |engine| {
        engine.memo.evaluation_work
    });
    let codes: Vec<_> = errors(&diagnostics)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, vec![crate::diagnostics::Code::RecursionDepth]);
    work
}

#[test]
fn reaching_the_nesting_limit_costs_work_linear_in_the_limit() {
    let (small, large) = (deepening_work(256), deepening_work(2_048));
    assert!(large <= 10 * small, "{small} {large}");
}

fn counted(bound: u64, limits: Limits) -> (Vec<Diagnostic>, Option<CVal>, u64) {
    let text = format!(
        "fn count(i) bounded {bound} else 0: count(i + 1) + 1\nClip c:\n    Note/(count(0) - {bound})\n"
    );
    let (diagnostics, (value, work)) = analyzed_within(&vocabulary(), limits, &text, |engine| {
        (
            attr_value(engine, "pitch", EvalTy::I32),
            engine.memo.evaluation_work,
        )
    });
    (diagnostics, value, work)
}

#[test]
fn a_bounded_function_evaluates_in_work_linear_in_its_levels() {
    let (small_diagnostics, small_value, small) = counted(25_000, Limits::default());
    let (large_diagnostics, large_value, large) = counted(100_000, Limits::default());
    assert!(
        errors(&small_diagnostics).is_empty(),
        "{small_diagnostics:?}"
    );
    assert!(
        errors(&large_diagnostics).is_empty(),
        "{large_diagnostics:?}"
    );
    assert_eq!(
        (small_value, large_value),
        (Some(CVal::Int(0)), Some(CVal::Int(0)))
    );
    assert!(large <= 4 * small, "{small} {large}");
    let limits = Limits {
        recursive_calls: 20_000,
        ..Limits::default()
    };
    let (diagnostics, _, work) = counted(2_000_000, limits);
    let codes: Vec<_> = errors(&diagnostics)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, vec![crate::diagnostics::Code::RecursionLimit]);
    assert!(work <= small, "{work} {small}");
}

fn read_walk(text: &str) -> u64 {
    let (diagnostics, work) = analyzed(&vocabulary(), text, |engine| engine.memo.read_walk);
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    work
}

fn walk_program(levels: usize, body: &str) -> String {
    format!(
        "input held: i32[..4] = []\nfn walk(i) bounded {levels} else 0:\n{body}Clip c:\n    Note/(walk(0))\n"
    )
}

fn chain_program(functions: usize) -> String {
    let mut text = String::from("input held: i32[..4] = []\n");
    for function in 0..functions {
        let rest = if function + 1 < functions {
            format!("a + chain{}(i + 1)", function + 1)
        } else {
            String::from("a")
        };
        text.push_str(&format!(
            "fn chain{function}(i):\n    let a = held[i]\n    {rest}\n"
        ));
    }
    text.push_str("Clip c:\n    Note/(chain0(0))\n");
    text
}

fn assert_linear(points: [(u64, u64); 3]) {
    let [(n1, w1), (n2, w2), (n3, w3)] = points;
    let first = (w2 - w1) * (n3 - n2);
    let second = (w3 - w2) * (n2 - n1);
    assert!(4 * second <= 5 * first, "{points:?}");
}

#[test]
fn ordering_the_bindings_of_an_expanded_program_is_linear_in_it() {
    let with_let = "    let a = held[i]\n    if i >= len(held) then a else a + walk(i + 1)\n";
    let without_let = "    if i >= len(held) then held[i] else held[i] + walk(i + 1)\n";
    let unexpected = "    let a = held[i]\n    if i >= len(held) then 0 else (if a > 0 then 1 else 0) + walk(i + 1)\n";
    for body in [with_let, unexpected] {
        assert_linear([800, 1_600, 6_400].map(|levels| {
            (
                levels,
                read_walk(&walk_program(usize::try_from(levels).unwrap_or(0), body)),
            )
        }));
    }
    let plain = read_walk(&walk_program(6_400, without_let));
    let held = read_walk(&walk_program(6_400, with_let));
    assert!(held <= 4 * plain, "{plain} {held}");
    assert_linear([1_000, 2_000, 4_000].map(|functions| {
        (
            functions,
            read_walk(&chain_program(usize::try_from(functions).unwrap_or(0))),
        )
    }));
}

fn in_every_order(items: &[&str]) -> Vec<String> {
    let mut orders: Vec<Vec<&str>> = vec![Vec::new()];
    for item in items {
        orders = orders
            .into_iter()
            .flat_map(|order| {
                (0..=order.len()).map(move |position| {
                    let mut next = order.clone();
                    next.insert(position, item);
                    next
                })
            })
            .collect();
    }
    orders.into_iter().map(|order| order.concat()).collect()
}

fn passed_over_expansion_nodes(scrutinee: &[&str], bound: u32) -> Vec<(String, usize)> {
    let function = format!(
        "fn pick(i) bounded {bound} else 0:\n    match w:\n        Saw: held[0]\n        else: pick(i + 1) + pick(i + 2)\n"
    );
    let mut items = vec!["input held = [1, 2, 3]\n", function.as_str()];
    items.extend_from_slice(scrutinee);
    items.push("Clip c:\n    Note/(pick(0))\n");
    in_every_order(&items)
        .into_iter()
        .map(|text| {
            let (diagnostics, nodes) =
                analyzed_within(&vocabulary(), Limits::default(), &text, |engine| {
                    engine.nodes.len()
                });
            assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
            (text, nodes)
        })
        .collect()
}

#[test]
fn a_compile_time_match_in_an_expansion_expands_only_its_chosen_arm() {
    let scrutinees: [&[&str]; 2] = [
        &["let w = Saw\n", "Osc o | wave=w\n"],
        &["let w = Wave::Saw\n"],
    ];
    for scrutinee in scrutinees {
        let small = passed_over_expansion_nodes(scrutinee, 4);
        let large = passed_over_expansion_nodes(scrutinee, 12);
        for ((text, small), (_, large)) in small.iter().zip(&large) {
            assert_eq!(small, large, "{text}");
        }
    }
}

#[test]
fn a_compile_time_match_on_a_bare_variant_computes_its_chosen_arm() {
    let items = [
        "let w = Sine\n",
        "Osc o | wave=w\n",
        "let m = match w:\n    Saw: [1, 2]\n    Sine: [1, 2, 3]\n    else: [1]\n",
        "Clip c:\n    Note/(len(m) + m[2])\n",
    ];
    for text in in_every_order(&items) {
        let (diagnostics, value) = analyzed(&vocabulary(), &text, |engine| {
            attr_value(engine, "pitch", EvalTy::I32)
        });
        assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
        assert_eq!(value, Some(CVal::Int(6)), "{text}");
    }
}

#[test]
fn a_choice_whose_arms_agree_in_shape_computes_its_chosen_arm() {
    let held = "input held: i32[..4] = [1, 2]\n";
    let f = "fn f(n): if n > 3 then [1] else [1] + f(n + 1)\n";
    let rows: [(&[&str], i128); 3] = [
        (&[held, "Clip c:\n    Note/(if false then 7 else 5)\n"], 5),
        (&[f, "Clip c:\n    Note/(f(0)[0])\n"], 1),
        (
            &[f, "Clip c:\n    Note/((if false then f(0) else [1])[0])\n"],
            1,
        ),
    ];
    for (items, expected) in rows {
        for text in in_every_order(items) {
            let (diagnostics, value) = analyzed(&vocabulary(), &text, |engine| {
                attr_value(engine, "pitch", EvalTy::I32)
            });
            assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
            assert_eq!(value, Some(CVal::Int(expected)), "{text}");
        }
    }
}

#[test]
fn a_compile_time_index_into_a_literal_computes_its_element_in_the_positions_type() {
    let (diagnostics, value) = analyzed(
        &vocabulary(),
        "Clip c:\n    Note/(60 + [1, 2][1])\n",
        |engine| attr_value(engine, "pitch", EvalTy::I32),
    );
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert_eq!(value, Some(CVal::Int(62)));
}

#[test]
fn a_qualified_variant_in_an_array_that_only_len_reads_computes_its_length() {
    for text in in_every_order(&[
        "input w = Wave::Saw\n",
        "Clip c:\n    Note/(60 + len([w]))\n",
    ]) {
        let (diagnostics, value) = analyzed(&vocabulary(), &text, |engine| {
            attr_value(engine, "pitch", EvalTy::I32)
        });
        assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
        assert_eq!(value, Some(CVal::Int(61)), "{text}");
    }
}

#[test]
fn a_call_beside_the_inline_limit_computes_its_value() {
    let four = Limits {
        inlined_calls: 4,
        ..Limits::default()
    };
    let rows: [(&[&str], Limits, i128); 7] = [
        (
            &[
                "let fixed = [1, 2]\n",
                "fn r(i): if i >= len(fixed) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            four,
            62,
        ),
        (
            &[
                "input note = 60\n",
                "let cells = [note, note, note]\n",
                "fn r(i): if i >= len(cells) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            four,
            63,
        ),
        (
            &[
                "input held: i32[..4] = []\n",
                "let cells = [len(held), 1]\n",
                "fn r(i): if i >= len(cells) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            four,
            62,
        ),
        (
            &["fn h(x): x + 1\n", "Clip c:\n    Note/(h(h(h(60))))\n"],
            four,
            63,
        ),
        (
            &[
                "fn r(i): if i is 0 then 0 else r(i - 1) + 1\n",
                "Clip c:\n    Note/(60 + r(3))\n",
            ],
            Limits::default(),
            63,
        ),
        (
            &[
                "input fixed: i32[2] = [1, 2]\n",
                "fn r(i): if i >= len(fixed) then 0 else r(i + 1) + 1\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            four,
            62,
        ),
        (
            &[
                "input fixed: i32[2] = [1, 2]\n",
                "fn h(x): x + 1\n",
                "fn r(i): if i >= len(fixed) then 0 else r(i + 1) + h(h(h(i)))\n",
                "Clip c:\n    Note/(60 + r(0))\n",
            ],
            four,
            67,
        ),
    ];
    for (items, limits, pitch) in rows {
        for text in in_every_order(items) {
            let (diagnostics, value) = analyzed_within(&vocabulary(), limits, &text, |engine| {
                attr_value(engine, "pitch", EvalTy::I32)
            });
            assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
            assert_eq!(value, Some(CVal::Int(pitch)), "{text}");
        }
    }
}

#[test]
fn a_fixed_type_operand_widens_into_the_expected_type_before_the_operation() {
    let pitches: [(&[&str], i128); 9] = [
        (&["Clip c:\n    Note/(u8(255) + 4)\n"], 259),
        (
            &[
                "let packed = bytes([300, -5, 70000, 12])\n",
                "Clip c:\n    Note/(packed[0] + len(packed))\n",
            ],
            259,
        ),
        (
            &["fn g(x): x - u8(5)\n", "Clip c:\n    Note/(g(u8(3)))\n"],
            -2,
        ),
        (&["Clip c:\n    Note/(-u8(3))\n"], -3),
        (
            &["Clip c:\n    Note/(if true then u8(255) + 4 else 0)\n"],
            259,
        ),
        (
            &[
                "let m = match 1:\n    1: u8(255) + 4\n    else: 0\n",
                "Clip c:\n    Note/m\n",
            ],
            259,
        ),
        (
            &["let xs = [u8(255) + 4, 1]\n", "Clip c:\n    Note/(xs[0])\n"],
            259,
        ),
        (&["Clip c:\n    Note/(max(u8(255) + 4, 0))\n"], 259),
        (&["Clip c:\n    Note/((u8(255) + 4) div 2)\n"], 129),
    ];
    for (items, expected) in pitches {
        for text in in_every_order(items) {
            let (diagnostics, value) = analyzed(&vocabulary(), &text, |engine| {
                attr_value(engine, "pitch", EvalTy::I32)
            });
            assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
            assert_eq!(value, Some(CVal::Int(expected)), "{text}");
        }
    }
    let frequencies: [(&str, f64); 4] = [
        ("2147483647i32 + 1", 2_147_483_648.0),
        ("3.4028235e38f32 * 2", f64::from(f32::MAX) * 2.0),
        ("u8(250) * 2 / 2", 250.0),
        ("sqrt(u8(250) * 2)", 500.0_f64.sqrt()),
    ];
    for (value, expected) in frequencies {
        let text = format!("Osc o | freq=({value})\n");
        let (diagnostics, found) = analyzed(&vocabulary(), &text, |engine| {
            attr_value(engine, "freq", EvalTy::F64)
        });
        assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
        assert_eq!(found, Some(CVal::F64(expected)), "{text}");
    }
}

fn orders(items: &[&str]) -> Vec<String> {
    let mut found = vec![items.concat()];
    let mut order: Vec<&str> = items.to_vec();
    let mut counters = vec![0; items.len()];
    let mut level = 0;
    while level < items.len() {
        if counters[level] < level {
            let other = if level % 2 == 0 { 0 } else { counters[level] };
            order.swap(other, level);
            found.push(order.concat());
            counters[level] += 1;
            level = 0;
        } else {
            counters[level] = 0;
            level += 1;
        }
    }
    found
}

fn product_types(text: &str) -> (Option<Scalar>, Vec<Option<Scalar>>, Option<Scalar>) {
    let (diagnostics, found) = analyzed(&vocabulary(), text, |engine| {
        let product = engine
            .nodes
            .iter()
            .position(|node| matches!(node.op, Op::Binary(BinaryOp::Mul, ..)))
            .and_then(|index| u32::try_from(index).ok())
            .map(NodeId)
            .expect("the program holds the product");
        let checked: Vec<Option<Scalar>> = engine
            .typing
            .checked
            .iter()
            .filter(|(node, _, _)| *node == product)
            .map(|(_, ty, _)| *ty)
            .collect();
        let own = engine
            .bindings
            .iter()
            .position(|binding| binding.name == "m")
            .and_then(|index| u32::try_from(index).ok())
            .and_then(|index| engine.typing.own.get(&BindingId(index)).copied());
        (
            engine.typing.synth.get(&product).copied().flatten(),
            checked,
            own,
        )
    });
    assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
    found
}

#[test]
fn a_flexible_float_meeting_a_fixed_u8_operand_computes_in_f32() {
    let input = "input b: u8 = 3\n";
    let rows: &[(&[&str], Scalar)] = &[
        (
            &[input, "Clip c:\n    Note when b * 0.5 > 1\n"],
            Scalar::F32,
        ),
        (&["Clip c:\n    Note when u8(3) * 0.5 > 1\n"], Scalar::F32),
        (
            &["input n = 4\n", "Clip c:\n    Note when n * 0.5 > 1\n"],
            Scalar::F64,
        ),
    ];
    for (items, product) in rows {
        for text in orders(items) {
            let (synth, _, _) = product_types(&text);
            assert_eq!(synth, Some(*product), "{text}");
        }
    }
    for text in orders(&[input, "let m = b * 0.5\n", "Clip c:\n    Note when m > 1\n"]) {
        let (_, _, own) = product_types(&text);
        assert_eq!(own, Some(Scalar::F32), "{text}");
    }
    for text in orders(&[input, "Synth s:\n    Osc | freq=(b * 0.5)\n"]) {
        let (_, checked, _) = product_types(&text);
        assert_eq!(checked, vec![Some(Scalar::F64)], "{text}");
    }
}

#[test]
fn an_operator_over_recursive_calls_computes_with_the_shapes_their_results_settle_to() {
    let doubling = "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n";
    let growing = "fn g(n): if n is 0 then [1] else g(n - 1) + [n]\n";
    let bounded = "fn f(n) bounded 3 else [1]: f(n - 1) + f(n - 1)\n";
    let pitches: &[(&[&str], i128)] = &[
        (&[doubling, "Clip c:\n    Note/(60 + len(f(3)))\n"], 68),
        (&[growing, "Clip c:\n    Note/(60 + g(3)[3])\n"], 63),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + g(n)\n",
                "fn g(n): if n is 0 then [2] else f(n - 1)\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            64,
        ),
        (&[bounded, "Clip c:\n    Note/(60 + len(f(0)))\n"], 68),
        (
            &[
                "fn f(n) bounded 3 else [1]: f(n - 1) + f(n - 2)\n",
                "Clip c:\n    Note/(60 + len(f(0)))\n",
            ],
            68,
        ),
        (
            &[
                "fn s(n): if n is 0 then 1 else s(n - 1) + s(n - 1)\n",
                "Clip c:\n    Note/(60 + s(3))\n",
            ],
            68,
        ),
        (
            &[
                "fn f(n) bounded 3 else 0: f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note/(60 + f(0))\n",
            ],
            60,
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + [len(f(n - 1))]\n",
                "Clip c:\n    Note/(60 + f(3)[3])\n",
            ],
            63,
        ),
        (
            &[
                "fn f(n): if n is 0 then [5] else [f(n - 1)[^1] + 1]\n",
                "Clip c:\n    Note/(60 + f(3)[0])\n",
            ],
            68,
        ),
        (
            &[
                "fn f(n): if n is 0 then [1, 2] else (f(n - 1) + f(n - 1))[1..]\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            65,
        ),
        (
            &[
                "fn f(n): if n is 0 then [1] else f(n - 1) + [len(bytes(f(n - 1)) + [n])]\n",
                "Clip c:\n    Note/(60 + f(2)[2])\n",
            ],
            63,
        ),
        (
            &[
                "fn f(n): if n is 0 then [300] else bytes(f(n - 1)) + [n]\n",
                "Clip c:\n    Note/(60 + len(f(2)))\n",
            ],
            63,
        ),
        (
            &[growing, "Clip c:\n    Note/(60 + len(g(2) + g(1)))\n"],
            65,
        ),
        (
            &[
                growing,
                "let a = g(2) + g(1)\n",
                "Clip c:\n    Note/(60 + a[2])\n",
            ],
            62,
        ),
        (
            &[
                "fn f(n): if n is 0 then [[1, 2]] else f(n - 1) + f(n - 1)\n",
                "Clip c:\n    Note/(60 + f(2)[3][1])\n",
            ],
            62,
        ),
        (
            &[
                "fn f(n): if n is 0 then [[1, 2]] else [f(n - 1)[0] + f(n - 1)[0]]\n",
                "Clip c:\n    Note/(60 + len(f(2)[0]))\n",
            ],
            68,
        ),
        (
            &[
                "input i = 2\n",
                bounded,
                "Clip c:\n    Note/(60 + len(f(i)))\n",
            ],
            68,
        ),
    ];
    for (items, expected) in pitches {
        for text in in_every_order(items) {
            let (diagnostics, value) = analyzed(&vocabulary(), &text, |engine| {
                attr_value(engine, "pitch", EvalTy::I32)
            });
            assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
            assert_eq!(value, Some(CVal::Int(*expected)), "{text}");
        }
    }
}

fn doubling_work(read: &str, depth: u32) -> (Option<CVal>, u64, u64, u64) {
    let text = format!(
        "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\nClip c:\n    Note/({})\n",
        read.replace('k', &depth.to_string())
    );
    let (built_before, visited_before) = CArray::work();
    let (diagnostics, (value, evaluation)) = analyzed(&vocabulary(), &text, |engine| {
        (
            attr_value(engine, "pitch", EvalTy::I32),
            engine.memo.evaluation_work,
        )
    });
    let (built_after, visited_after) = CArray::work();
    assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
    (
        value,
        built_after - built_before,
        visited_after - visited_before,
        evaluation,
    )
}

#[test]
fn a_join_holds_its_operands_and_builds_its_elements_only_where_a_value_needs_them() {
    for read in ["60 + len(f(k))", "60 + f(k)[5]"] {
        let first = doubling_work(read, 10).1;
        for depth in [20, 30] {
            assert_eq!(doubling_work(read, depth).1, first, "{read} at {depth}");
        }
    }
    let (length, _, _, _) = doubling_work("60 + len(f(k))", 30);
    assert_eq!(length, Some(CVal::Int(1_073_741_884)));
    let (element, _, _, _) = doubling_work("60 + f(k)[5]", 30);
    assert_eq!(element, Some(CVal::Int(61)));
    let work: Vec<(u64, u64)> = [10, 20, 30]
        .into_iter()
        .map(|depth| {
            let (_, _, visited, evaluation) = doubling_work("60 + len(f(k))", depth);
            (visited, evaluation)
        })
        .collect();
    for pair in work.windows(2) {
        let (earlier, later) = (pair[0], pair[1]);
        assert_eq!(later.0 - earlier.0, work[1].0 - work[0].0, "{work:?}");
        assert_eq!(later.1 - earlier.1, work[1].1 - work[0].1, "{work:?}");
    }
    let ragged_built: Vec<u64> = [3, 30]
        .into_iter()
        .map(|bound| {
            let text = format!(
                "fn f(n) bounded {bound} else [[1, 2]]: f(n - 1) + [[3]]\nClip c:\n    Note/(60 + len(f(0)))\n"
            );
            let (built_before, _) = CArray::work();
            let (diagnostics, ()) = analyzed(&vocabulary(), &text, |_| ());
            let (built_after, _) = CArray::work();
            assert_eq!(errors(&diagnostics).len(), 1, "{text}{diagnostics:?}");
            built_after - built_before
        })
        .collect();
    assert_eq!(ragged_built[0], ragged_built[1]);
    let doubling = "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n";
    for text in in_every_order(&[
        doubling,
        "let p: i32[8] = f(3)\n",
        "Clip c:\n    Note/(p[7])\n",
    ]) {
        let (diagnostics, value) = analyzed(&vocabulary(), &text, |engine| {
            attr_value(engine, "pitch", EvalTy::I32)
        });
        assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
        assert_eq!(value, Some(CVal::Int(1)), "{text}");
    }
}

#[test]
fn a_compile_time_slice_holds_its_base_and_its_bounds() {
    let (_, reference, _, _) = doubling_work("60 + len(f(k))", 30);
    for (read, expected) in [
        ("60 + len(f(k)[5..])", 1_073_741_879),
        ("60 + len(f(k)[5..^9])", 1_073_741_870),
        ("60 + len(f(k)[5..9])", 64),
    ] {
        let (value, built, _, _) = doubling_work(read, 30);
        assert_eq!(value, Some(CVal::Int(expected)), "{read}");
        assert_eq!(built, reference, "{read}");
    }
    let (element, _, _, _) = doubling_work("f(k)[1..3][1]", 3);
    assert_eq!(element, Some(CVal::Int(1)));
    for text in in_every_order(&[
        "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n",
        "let p: i32[1073741824] = f(30)\n",
        "Clip c:\n    Note/(p[0..])\n",
    ]) {
        let (built_before, _) = CArray::work();
        let (diagnostics, ()) = analyzed(&vocabulary(), &text, |_| ());
        let (built_after, _) = CArray::work();
        let found: Vec<crate::diagnostics::Code> = errors(&diagnostics)
            .iter()
            .map(|found| found.code)
            .collect();
        assert_eq!(found, [crate::diagnostics::Code::TypeMismatch], "{text}");
        assert!(built_after - built_before <= reference, "{text}");
    }
}

#[test]
fn a_bounded_expansion_types_its_passed_over_branches_in_work_linear_in_its_levels() {
    let nodes = |bound: u32| {
        let text = format!(
            "input held: i32[..4] = [1, 2]\nfn f(i) bounded {bound} else 0: if i > 1000 then f(i + 1) + f(i + 2) else held[0] + f(i + 1)\nClip c:\n    Note/(f(0))\n"
        );
        let (diagnostics, nodes) = analyzed(&vocabulary(), &text, |engine| engine.nodes.len());
        assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
        nodes
    };
    let counts = [nodes(100), nodes(200), nodes(300)];
    assert_eq!(counts[2] - counts[1], counts[1] - counts[0], "{counts:?}");
}

fn set_attr_value(engine: &Engine<'_>, name: &str, ty: EvalTy) -> Option<CVal> {
    engine.instances.iter().find_map(|instance| {
        let decl = engine.vocabulary.node_kind(instance.kind)?;
        let source = instance.attrs.iter().find(|source| {
            decl.attrs
                .get(source.attr.index())
                .is_some_and(|attr| attr.name == name)
        })?;
        let AttrValueSource::Expr(expr, scope) = source.value else {
            return None;
        };
        let Some(Val::Scalar(node)) = engine.memo.exprs.get(&(expr, scope)) else {
            return None;
        };
        engine.memo.evals.get(&(*node, ty)).cloned()
    })
}

#[test]
fn an_unset_variable_array_attribute_reads_as_the_fixed_array_of_its_default() {
    let unset = "Clip b:\n    Note\n";
    let seq = "Clip b | seq=[7, 8]:\n    Note\n";
    let rows: [(&[&str], &str, EvalTy, CVal); 9] = [
        (
            &["Clip a:\n    Note/(60 + len(b.seq))\n", unset],
            "pitch",
            EvalTy::I32,
            CVal::Int(62),
        ),
        (
            &["Clip a:\n    Note/(60 + len(b.seq))\n", seq],
            "pitch",
            EvalTy::I32,
            CVal::Int(62),
        ),
        (
            &[
                "Clip c:\n    Note/(len(d.sequence))\n",
                "Clip d:\n    Note\n",
            ],
            "pitch",
            EvalTy::I32,
            CVal::Int(0),
        ),
        (
            &["Clip a | gain=len(b.seq):\n    Note\n", unset],
            "gain",
            EvalTy::F32,
            CVal::F32(2.0),
        ),
        (
            &["Clip a:\n    Note/(b.seq[1])\n", unset],
            "pitch",
            EvalTy::I32,
            CVal::Int(8),
        ),
        (
            &["Clip a:\n    Note/(b.seq[1])\n", seq],
            "pitch",
            EvalTy::I32,
            CVal::Int(8),
        ),
        (
            &["Clip a:\n    Note/(len(b.rows))\n", unset],
            "pitch",
            EvalTy::I32,
            CVal::Int(2),
        ),
        (
            &["Clip a:\n    Note/(b.rows[1][0])\n", unset],
            "pitch",
            EvalTy::I32,
            CVal::Int(3),
        ),
        (
            &["Clip c | sequence=[1, 2]:\n    Note/(len(c.sequence))\n"],
            "pitch",
            EvalTy::I32,
            CVal::Int(2),
        ),
    ];
    for (items, name, ty, expected) in rows {
        for text in in_every_order(items) {
            let (diagnostics, value) = analyzed(&native_vocabulary(), &text, |engine| {
                set_attr_value(engine, name, ty)
            });
            assert!(errors(&diagnostics).is_empty(), "{text}{diagnostics:?}");
            assert_eq!(value, Some(expected.clone()), "{text}");
        }
    }
}

fn layout_visits(text: &str) -> u64 {
    let (diagnostics, visits) = analyzed(&vocabulary(), text, |engine| engine.memo.layout_visits);
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    visits
}

#[test]
fn a_layout_walk_over_a_chain_of_slices_costs_work_linear_in_the_chain() {
    type Chain = fn(usize) -> String;
    let chains: [Chain; 3] = [
        |n| {
            format!(
                "input note = 60\nstate s = (s{{1, []}} + [note]){}\nClip c:\n    Note/(s[0])\n",
                "[^8..]".repeat(n)
            )
        },
        |n| {
            format!(
                "input note = 60\nstate s = [note, note, note]\nlet t = s{}\nClip c:\n    Note/(len(t) + note)\n",
                "[1..]".repeat(n)
            )
        },
        |n| {
            format!(
                "input held = [1, 2, 3]\nstate s = held{{1}}{}\nClip c:\n    Note/(len(s))\n",
                "[1..]".repeat(n)
            )
        },
    ];
    for chain in chains {
        let visits: Vec<u64> = [2_500, 5_000, 10_000]
            .into_iter()
            .map(|length| layout_visits(&chain(length)))
            .collect();
        for pair in visits.windows(2) {
            if let [shorter, longer] = pair {
                assert!(*longer <= 2 * *shorter + 64, "{visits:?}");
            }
        }
    }
}

fn big(args: &[symphonyscript_core::NativeArg<'_>], out: &mut symphonyscript_core::NativeOut<'_>) {
    if let Some(symphonyscript_core::NativeArg::Value(value)) = args.first() {
        out.push(*value);
    }
}

fn big_vocabulary() -> symphonyscript_core::Vocabulary {
    use symphonyscript_core::{Count, NativeSignature, NativeType, Ty, Vocabulary};
    let mut v = Vocabulary::builder("big");
    let clip = v.node("Clip");
    let note = v.node("Note");
    v.kind(clip)
        .top_level(Count::Any)
        .children(note, Count::Any);
    v.kind(note).attr::<i32>("pitch").default(60).default_attr();
    v.native(
        "big",
        NativeSignature::new(
            [NativeType::Value(Ty::I32)],
            NativeType::Value(Ty::I32.variable(2_147_483_647)),
        ),
        big,
    );
    v.build().expect("the big vocabulary builds")
}

#[test]
fn a_compile_time_native_call_costs_what_it_writes() {
    let vocabulary = big_vocabulary();
    let before = super::evaluation::native_slots();
    let (diagnostics, pitch) = analyzed(&vocabulary, "Clip c:\n    Note/(big(1)[0])\n", |engine| {
        attr_value(engine, "pitch", EvalTy::I32)
    });
    let slots = super::evaluation::native_slots() - before;
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert_eq!(pitch, Some(CVal::Int(1)));
    assert!(slots <= 1 + 8, "{slots}");
    let text = "Clip c:\n    for i in 0..1000:\n        Note/(big(i)[0])\n";
    let (diagnostics, notes) = analyzed(&vocabulary, text, |engine| {
        let note = engine.vocabulary.node_kind_named("Note");
        engine
            .instances
            .iter()
            .filter(|instance| Some(instance.kind) == note)
            .count()
    });
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert_eq!(notes, 1000);
}

fn flow_vocabulary() -> symphonyscript_core::Vocabulary {
    let mut v = symphonyscript_core::Vocabulary::builder("flow cost");
    let wave = v.enumeration("Wave", ["Sine", "Saw"]);
    let color = v.enumeration("Color", ["Red", "Blue"]);
    let both = v.node("K");
    v.kind(both)
        .top_level(symphonyscript_core::Count::Any)
        .attr_enum("wave", &wave, "Sine")
        .attr_enum("color", &color, "Red");
    v.build().expect("the flow cost vocabulary builds")
}

fn flow_program(uses: u64) -> String {
    let mut text =
        String::from("pattern P():\n    let a = Sine\n    let b = a\n    K | wave=a color=b\n");
    for use_ in 0..uses {
        text.push_str(&format!("P p{use_}\n"));
    }
    text
}

#[test]
fn erroneous_flows_report_in_work_linear_in_their_number() {
    let vocabulary = flow_vocabulary();
    let points = [2_000, 4_000, 8_000].map(|uses| {
        let (diagnostics, work) = analyzed(&vocabulary, &flow_program(uses), |engine| {
            (
                engine.memo.flow_work.get(),
                engine.memo.query_runs,
                u64::try_from(engine.nodes.len()).unwrap_or(u64::MAX),
            )
        });
        assert_eq!(
            u64::try_from(errors(&diagnostics).len()).unwrap_or(u64::MAX),
            uses,
            "one diagnostic per use"
        );
        (uses, work)
    });
    assert_linear(points.map(|(uses, (flow, _, _))| (uses, flow)));
    assert_linear(points.map(|(uses, (_, queries, _))| (uses, queries)));
    assert_linear(points.map(|(uses, (_, _, nodes))| (uses, nodes)));
}

fn three_vocabulary() -> symphonyscript_core::Vocabulary {
    let mut v = symphonyscript_core::Vocabulary::builder("three");
    let clip = v.node("Clip");
    let note = v.node("Note");
    v.kind(clip)
        .top_level(symphonyscript_core::Count::Any)
        .children(note, symphonyscript_core::Count::Any)
        .attr::<symphonyscript_core::Bounded<i32, 3>>("three")
        .default(symphonyscript_core::Bounded::default());
    v.kind(note).attr::<i32>("pitch").default(60).default_attr();
    v.build().expect("the three vocabulary builds")
}

fn joined_let(name: &str, joins: u32) -> String {
    format!(
        "input held: i32[..1] = []\nlet {name} = held{}\n",
        " + [1]".repeat(usize::try_from(joins).unwrap_or(0))
    )
}

fn attribute_walk(positions: u32, joins: u32) -> u64 {
    let mut text = joined_let("x", joins);
    for position in 0..positions {
        text.push_str(&format!("Clip c{position} | three=x:\n    Note\n"));
    }
    let (diagnostics, visits) = analyzed(&three_vocabulary(), &text, |engine| {
        engine.memo.capacity_visits
    });
    let found = errors(&diagnostics);
    assert_eq!(found.len(), usize::try_from(positions).unwrap_or(0));
    let holds = format!("holds up to {} elements", joins + 1);
    for diagnostic in found {
        assert_eq!(diagnostic.code, crate::diagnostics::Code::CapacityExceeded);
        assert!(diagnostic.summary.contains(&holds), "{diagnostic:?}");
    }
    visits
}

fn state_walk(states: u32, joins: u32, history: bool) -> u64 {
    let mut text = joined_let("t", joins);
    let read = if history { "t{1, []}" } else { "t" };
    for state in 0..states {
        text.push_str(&format!("state s{state} = {read}[^2..]\n"));
    }
    text.push_str("Clip c:\n    Note/(s0[0])\n");
    let (diagnostics, (visits, capacities)) = analyzed(&vocabulary(), &text, |engine| {
        let capacities: Vec<Option<u32>> = (0..states)
            .map(|state| state_capacity(engine, state))
            .collect();
        (
            engine.memo.settlement_visits + engine.memo.capacity_visits,
            capacities,
        )
    });
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert!(
        capacities.iter().all(|capacity| *capacity == Some(2)),
        "{capacities:?}"
    );
    visits
}

#[test]
fn a_lets_capacity_is_walked_once_however_many_positions_or_states_read_it() {
    let small = attribute_walk(1_000, 1_000);
    let large = attribute_walk(2_000, 2_000);
    assert!(large <= 2 * small + 64, "{small} {large}");
    for history in [true, false] {
        let smallest = state_walk(1_000, 1_000, history);
        let base = state_walk(1_000, 2_000, history);
        let largest = state_walk(2_000, 2_000, history);
        let longer = state_walk(2_000, 1_000, history);
        for smaller in [smallest, base, longer] {
            assert!(
                largest <= 2 * smaller + 64,
                "history {history}: {smallest} {base} {longer} {largest}"
            );
        }
    }
}

#[test]
fn reachability_warnings_cost_work_linear_in_the_instances() {
    let work = |clips: usize| {
        let mut text = String::from("Session s\n");
        for clip in 0..clips {
            text.push_str(&format!("Clip c{clip}\n"));
        }
        let (diagnostics, work) = analyzed(&vocabulary(), &text, |engine| engine.memo.reach_work);
        assert_eq!(diagnostics.len(), clips, "{diagnostics:?}");
        work
    };
    let (small, large) = (work(10_000), work(40_000));
    assert!(large <= 4 * small + 64, "{small} {large}");
}

#[test]
fn reporting_reachability_warnings_costs_work_linear_in_the_warnings() {
    for clips in [1_000u64, 2_000, 4_000] {
        let mut text = String::from("Session s\n");
        for clip in 0..clips {
            text.push_str(&format!("Clip c{clip}\n"));
        }
        let (diagnostics, reports) =
            analyzed(&vocabulary(), &text, |engine| engine.memo.reach_reports);
        let warnings = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == crate::diagnostics::Code::UnreachableInstance)
            .count();
        assert_eq!(warnings, usize::try_from(clips).unwrap_or(usize::MAX));
        assert_eq!(diagnostics.len(), warnings, "{diagnostics:?}");
        let per_instance = 2;
        assert!(reports <= per_instance * (clips + 1), "{clips} {reports}");
    }
}

#[test]
fn an_integer_that_would_reach_two_to_the_127_costs_what_its_reference_costs() {
    let work = |text: &str| {
        let (diagnostics, work) =
            analyzed(&vocabulary(), text, |engine| engine.memo.evaluation_work);
        assert_eq!(errors(&diagnostics).len(), 1, "{text}{diagnostics:?}");
        work
    };
    let just_under = "170141183460469231731687303715884105727";
    let reference = work("Clip c:\n    Note/(2 ** 127)\n");
    for text in [
        "Clip c:\n    Note/(2 ** 1000000000)\n".to_string(),
        "let e = 2 ** 2 ** 2 ** 2 ** 2\nClip c:\n    Note/e\n".to_string(),
        format!("Clip c:\n    Note/({just_under} * {just_under})\n"),
    ] {
        assert!(work(&text) <= 10 * reference.max(1), "{text}");
    }
}

#[test]
fn variant_lookups_cost_constant_work_per_name() {
    let vocabulary_over = |names: Vec<String>| {
        let mut v = symphonyscript_core::Vocabulary::builder("wide");
        let variants = v.enumeration("E", &names);
        let other = v.enumeration("F", ["Other"]);
        let kind = v.node("K");
        v.kind(kind)
            .top_level(symphonyscript_core::Count::Any)
            .attr_enum("e", variants, "V65535")
            .attr_enum("f", other, "Other");
        v.build().expect("the vocabulary builds")
    };
    let named = |range: std::ops::Range<usize>| range.map(|index| format!("V{index}")).collect();
    let mut text = String::new();
    for index in 0..20_000 {
        text.push_str(&format!("K k{index} | e=V{}\n", 65_472 + index % 64));
    }
    text.push_str("K x | e=Other\n");
    let examined = |names: Vec<String>| {
        let (diagnostics, examined) = analyzed(&vocabulary_over(names), &text, |engine| {
            engine.variants.examined()
        });
        assert_eq!(errors(&diagnostics).len(), 1, "{diagnostics:?}");
        examined
    };
    let wide = examined(named(0..65_536));
    let narrow = examined(named(65_472..65_536));
    assert_eq!(wide, narrow);
    assert!(wide <= 8 * 20_001, "{wide}");
}
