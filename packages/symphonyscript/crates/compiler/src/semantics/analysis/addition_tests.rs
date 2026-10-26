use symphonyscript_core::Limits;
use symphonyscript_fixtures::{native_vocabulary, vocabulary};

use super::Engine;
use super::attr_value_source::AttrValueSource;
use super::bound::Bound;
use super::bound_val::BoundVal;
use super::c_val::CVal;
use super::eval_ty::EvalTy;
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
fn a_for_over_compile_time_values_expands_once_per_element() {
    let text = "fn g(x): x + 1\nClip c:\n    for i in [1, 2]:\n        Note\n    for i in [g(3)]:\n        Note\n";
    let (diagnostics, notes) = analyzed(&vocabulary(), text, |engine| {
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
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert_eq!(notes, 3);
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
