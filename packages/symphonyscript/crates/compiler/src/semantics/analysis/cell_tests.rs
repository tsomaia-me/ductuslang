use symphonyscript_core::Scalar;
use symphonyscript_fixtures::vocabulary;

use super::Engine;
use super::history_cell::HistoryCell;
use super::ids::BindingId;
use super::layout::Layout;
use crate::compiler::Compiler;
use crate::diagnostics::{Code, Severity};
use crate::semantics::ir::{CellWord, InputId, StateId};
use crate::source::Source;

fn analyzed<R>(text: &str, inspect: impl FnOnce(&Engine<'_>) -> R) -> (Vec<(Code, String)>, R) {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let (diagnostics, found) = compiler.analyzed(&[source], inspect);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.severity() == Severity::Error && diagnostic.code != Code::NoEntryInstance
        })
        .map(|diagnostic| {
            let range = &diagnostic.span.range;
            let spanned = text
                .get(range.start as usize..range.end as usize)
                .unwrap_or_default();
            (diagnostic.code, spanned.to_string())
        })
        .collect();
    (errors, found.expect("the run meets no internal error"))
}

fn binding_named(engine: &Engine<'_>, name: &str) -> Option<BindingId> {
    let position = engine
        .bindings
        .iter()
        .position(|binding| binding.name == name)?;
    Some(BindingId(u32::try_from(position).ok()?))
}

fn own(engine: &Engine<'_>, name: &str) -> Option<Scalar> {
    engine
        .typing
        .own
        .get(&binding_named(engine, name)?)
        .copied()
}

#[test]
fn the_cells_of_the_spec_take_the_types_and_capacities_the_spec_gives_them() {
    let cutoff = "state cutoff(0.5)\n\nClip melody:\n    Note | brightness=cutoff\n\nSynth lead:\n    Osc | freq=cutoff * 4000\n";
    let (errors, (cell, expected)) = analyzed(cutoff, |engine| {
        let binding = binding_named(engine, "cutoff");
        let expected: Vec<Scalar> = binding
            .and_then(|binding| engine.typing.expectations.get(&binding))
            .map(|uses| uses.iter().map(|(scalar, _)| *scalar).collect())
            .unwrap_or_default();
        (own(engine, "cutoff"), expected)
    });
    assert_eq!(errors, vec![]);
    assert_eq!(cell, Some(Scalar::F64));
    assert!(expected.contains(&Scalar::F32), "{expected:?}");
    let widths = "state a(1)\nstate b = a + 0.5\n\nClip c:\n    Note when b > 1\n";
    let (errors, cells) = analyzed(widths, |engine| (own(engine, "a"), own(engine, "b")));
    assert_eq!(errors, vec![]);
    assert_eq!(cells, (Some(Scalar::I64), Some(Scalar::F32)));
    let derived = "state gate(false)\nstate level(0.0)\nstate note(60)\n\nstate held(0.0) on gate = if gate then level else held.past(1)\nstate previous = note.past(1, -1)\nstate recent([]) = (recent.past(1) + [note])[^8..]\n";
    let (errors, (held, previous, recent)) = analyzed(derived, |engine| {
        let capacity = binding_named(engine, "recent")
            .and_then(|binding| engine.memo.binding_capacities.get(&binding).copied());
        (own(engine, "held"), own(engine, "previous"), capacity)
    });
    assert_eq!(
        errors,
        vec![(Code::UnbuiltConstruct, "on gate".to_string())]
    );
    assert_eq!(held, Some(Scalar::F32));
    assert_eq!(previous, Some(Scalar::I64));
    assert_eq!(recent, Some(8));
    let hits = "channel hit(0)\nstate bar(0)\n\nstate hits(0) on (hit, bar) = if bar.stepped then 0 else hits.past(1) + 1\n";
    let (errors, cell) = analyzed(hits, |engine| own(engine, "hits"));
    assert_eq!(
        errors,
        vec![(Code::UnbuiltConstruct, "on (hit, bar)".to_string())]
    );
    assert!(
        matches!(cell, Some(Scalar::U8 | Scalar::I32 | Scalar::I64)),
        "{cell:?}"
    );
}

#[test]
fn a_cell_carries_its_word_its_capacity_and_the_depth_of_its_deepest_past_read() {
    let text = "channel[16] notes(0)\nstream mic(0)\nstate level(1)\nstream d = level + 1\nchannel e = d.past(3, 0)\nClip c:\n    Note/(level.past(5) + notes + d.past(1) + e + mic.past(2))\n";
    let (errors, (inputs, states, depths)) = analyzed(text, |engine| {
        let inputs: Vec<(CellWord, Option<u32>)> = engine
            .hir
            .inputs
            .iter()
            .map(|input| (input.word, input.capacity))
            .collect();
        let states: Vec<CellWord> = engine.hir.states.iter().map(|decl| decl.word).collect();
        let depths = [
            HistoryCell::Input(InputId(0)),
            HistoryCell::Input(InputId(1)),
            HistoryCell::Input(InputId(2)),
            HistoryCell::State(StateId(0)),
            HistoryCell::State(StateId(1)),
        ]
        .map(|cell| engine.history_depth(cell));
        (inputs, states, depths)
    });
    assert_eq!(errors, vec![]);
    assert_eq!(
        inputs,
        vec![
            (CellWord::Channel, Some(16)),
            (CellWord::Stream, None),
            (CellWord::State, None),
        ]
    );
    assert_eq!(states, vec![CellWord::Stream, CellWord::Channel]);
    assert_eq!(depths, [0, 2, 5, 3, 0]);
    let inlined =
        "state note(1)\nfn id(x): x\nstate s = id(s.past(2, 0)) + note\nClip c:\n    Note/(s)\n";
    let (errors, depth) = analyzed(inlined, |engine| {
        engine.history_depth(HistoryCell::State(StateId(0)))
    });
    assert_eq!(errors, vec![]);
    assert_eq!(depth, 2);
}

fn in_both_orders(items: [&str; 2], rest: &str) -> [String; 2] {
    [
        format!("{}{}{rest}", items[0], items[1]),
        format!("{}{}{rest}", items[1], items[0]),
    ]
}

#[test]
fn a_history_read_with_neither_fallback_nor_init_takes_its_cells_type_once_the_group_settles() {
    for text in in_both_orders(
        ["state t(0) = r.past(1)\n", "state r = t + 1\n"],
        "Clip c:\n    Note when t > 1\n",
    ) {
        let (errors, cells) = analyzed(&text, |engine| (own(engine, "t"), own(engine, "r")));
        assert_eq!(errors, vec![], "{text}");
        assert_eq!(cells, (Some(Scalar::I32), Some(Scalar::I32)), "{text}");
    }
    for text in in_both_orders(
        ["state t(0) = r.past(1)\n", "state r(0.5) = t + 1\n"],
        "Clip c:\n    Note | brightness=r\n",
    ) {
        let (errors, cells) = analyzed(&text, |engine| (own(engine, "t"), own(engine, "r")));
        assert_eq!(errors, vec![], "{text}");
        assert!(
            matches!(
                cells,
                (
                    Some(Scalar::F32 | Scalar::F64),
                    Some(Scalar::F32 | Scalar::F64)
                )
            ),
            "{text}\n{cells:?}"
        );
    }
    let layout = |engine: &Engine<'_>, name: &str| {
        binding_named(engine, name)
            .and_then(|binding| engine.memo.binding_layouts.get(&binding).cloned())
    };
    let capacity = |engine: &Engine<'_>, name: &str| {
        binding_named(engine, name)
            .and_then(|binding| engine.memo.binding_capacities.get(&binding).copied())
    };
    for text in in_both_orders(
        [
            "state t = r.past(1)\n",
            "state r([1, 2]) = [t[1] + n, t[0]]\n",
        ],
        "state n(1)\nClip c:\n    Note/(t[0] + r[1])\n",
    ) {
        let (errors, layouts) =
            analyzed(&text, |engine| (layout(engine, "t"), layout(engine, "r")));
        assert_eq!(errors, vec![], "{text}");
        let fixed = Some(Layout::fixed(vec![2]));
        assert_eq!(layouts, (fixed.clone(), fixed), "{text}");
    }
    for text in in_both_orders(
        [
            "state t = r.past(1)\n",
            "state r([]) = (t + [note])[^4..]\n",
        ],
        "state note(60)\nClip c:\n    Note/(note)\n",
    ) {
        let (errors, held) = analyzed(&text, |engine| {
            (
                layout(engine, "t"),
                layout(engine, "r"),
                capacity(engine, "t"),
                capacity(engine, "r"),
            )
        });
        assert_eq!(errors, vec![], "{text}");
        let variable = Some(Layout::variable(Vec::new()));
        assert_eq!(
            held,
            (variable.clone(), variable, Some(4), Some(4)),
            "{text}"
        );
    }
    for text in in_both_orders(
        ["state t = r.past(1)\n", "state r = [n, n]\n"],
        "state n(1)\nClip c:\n    Note/(t[0])\n",
    ) {
        let (errors, held) = analyzed(&text, |engine| layout(engine, "t"));
        assert_eq!(errors, vec![], "{text}");
        assert_eq!(held, Some(Layout::fixed(vec![2])), "{text}");
    }
}
