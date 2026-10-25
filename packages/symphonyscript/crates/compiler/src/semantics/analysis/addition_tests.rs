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
    let compiler = Compiler::new(vocabulary).expect("the vocabulary compiles");
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
    let cases: [(&str, i128); 5] = [
        ("checksum(\"GET\")", 71 + 69 + 84),
        ("checksum([u8(1), u8(2), u8(3)])", 6),
        ("text_length(\"hello\")", 5),
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
fn a_bounded_call_yields_its_fallback_nested_inside_its_bound() {
    let text = "fn count(i) bounded 3 else 100: count(i + 1) + 1\nClip c:\n    Note/(count(0))\n";
    let (diagnostics, value) = analyzed(&vocabulary(), text, |engine| {
        attr_value(engine, "pitch", EvalTy::I32)
    });
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:?}");
    assert_eq!(value, Some(CVal::Int(103)));
}
