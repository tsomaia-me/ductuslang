use symphonyscript_fixtures::{
    Project, ProjectFile, deep_projects, dsp_project, music_project, spec_examples, spec_projects,
    vocabulary,
};

use super::Engine;
use super::attr_source::AttrValueSource;
use super::binding::BindingKind;
use super::bound::BoundVal;
use super::destination::Destination;
use super::ids::ScopeId;
use super::node::Op;
use super::val::Val;
use crate::compiler::Compiler;
use crate::diagnostics::{Diagnostic, Severity};
use crate::semantics::collection::collect;
use crate::semantics::ir::ExprId;
use crate::semantics::ir::{Hir, build_file};
use crate::semantics::reporting::Report;
use crate::source::Source;
use crate::syntax::parse;

fn graph_size(calls: usize) -> (usize, usize) {
    let text = format!(
        "input i = 1\nfn twice(x): x + x\nClip c:\n    Note/({}i{})\n",
        "twice(".repeat(calls),
        ")".repeat(calls)
    );
    let vocabulary = vocabulary();
    let source = Source {
        path: "chain.sym".into(),
        text,
    };
    let mut hir = Hir::default();
    hir.seal_prelude();
    let parsed = parse(&source, &vocabulary);
    build_file(&mut hir, &source.path, &parsed, &vocabulary);
    let mut report = Report::default();
    let collected = collect(&hir, &vocabulary, &mut report);
    let mut engine = Engine::new(&hir, &vocabulary, &collected);
    engine.expand();
    engine.infer();
    engine.check_program();
    let errors: Vec<_> = engine
        .report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    (engine.nodes.len(), engine.bindings.len())
}

#[test]
fn nested_calls_that_use_their_parameter_twice_grow_the_graph_linearly() {
    let (small, middle, large) = (graph_size(10), graph_size(20), graph_size(40));
    assert_eq!(large.0 - middle.0, 2 * (middle.0 - small.0), "nodes");
    assert_eq!(large.1 - middle.1, 2 * (middle.1 - small.1), "bindings");
    assert_eq!(middle.1 - small.1, 10, "one binding per call");
}

fn analyzed<R>(text: &str, inspect: impl FnOnce(&Engine<'_>) -> R) -> (Vec<Diagnostic>, R) {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    compiler.analyzed(&[source], inspect)
}

fn placeholders(engine: &Engine<'_>) -> Vec<String> {
    let memo = &engine.memo;
    let mut found = Vec::new();
    for ((expr, _), val) in &memo.exprs {
        if matches!(val, Val::Error) {
            found.push(format!("value of {:?}", engine.hir.expr(*expr).loc));
        }
    }
    for (node, sig) in &memo.sigs {
        if sig.is_error() {
            found.push(format!("type of {:?}", engine.node(*node).loc));
        }
    }
    for ((node, _), value) in &memo.evals {
        if value.is_error() {
            found.push(format!("evaluation of {:?}", engine.node(*node).loc));
        }
    }
    for (call, value) in &memo.calls {
        if value.is_error() {
            found.push(format!("result of {call:?}"));
        }
    }
    let bounds = memo
        .lets
        .values()
        .chain(memo.params.values())
        .chain(memo.inputs.values())
        .chain(memo.attrs.values());
    for bound in bounds {
        if matches!(bound, BoundVal::Error) {
            found.push("a binding".into());
        }
    }
    for destination in memo.destinations.values() {
        if matches!(destination, Destination::Error) {
            found.push("a destination".into());
        }
    }
    found
}

#[test]
fn a_run_that_draws_no_error_holds_no_placeholder() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let examples = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym")
        .map(|example| Project {
            name: example.name(),
            files: vec![ProjectFile {
                path: "example.sym".into(),
                text: example.text,
            }],
        });
    let projects = [music_project(), dsp_project()]
        .into_iter()
        .chain(deep_projects())
        .chain(examples);
    let clean = std::thread::scope(|threads| {
        let runs: Vec<_> = projects
            .map(|project| {
                let compiler = &compiler;
                threads.spawn(move || {
                    let sources: Vec<Source> = project
                        .files
                        .iter()
                        .map(|file| Source {
                            path: file.path.clone(),
                            text: file.text.clone(),
                        })
                        .collect();
                    let (diagnostics, found) = compiler.analyzed(&sources, placeholders);
                    let drew_error = !errors_of(&diagnostics).is_empty();
                    assert!(
                        drew_error || found.is_empty(),
                        "{}: {found:?}",
                        project.name
                    );
                    !drew_error
                })
            })
            .collect();
        runs.into_iter()
            .map(|run| run.join().expect("every run holds no placeholder"))
            .filter(|clean| *clean)
            .count()
    });
    assert!(
        clean >= 6 + spec_projects().len(),
        "{clean} runs drew no error"
    );
}

fn errors_of(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .collect()
}

fn reads(engine: &Engine<'_>, expr: ExprId, scope: ScopeId) -> String {
    let Some(Val::Scalar(mut node)) = engine.memo.exprs.get(&(expr, scope)).cloned() else {
        return "no single value".into();
    };
    loop {
        match engine.node(node).op {
            Op::Use(inner) => node = inner,
            Op::Read(binding) => {
                let record = engine.binding(binding);
                return match record.kind {
                    BindingKind::Input { input, element } => {
                        let name = engine.hir.input(input).name.as_ref();
                        format!("input {}[{element}]", name.map_or("", |name| &name.text))
                    }
                    _ => format!("cell {}", record.name),
                };
            }
            Op::Pick { .. } => return "runtime pick".into(),
            _ => return "computed".into(),
        }
    }
}

fn note_pitches_and_gates(engine: &Engine<'_>) -> Vec<(String, Option<String>)> {
    engine
        .instances
        .iter()
        .filter(|instance| {
            engine
                .vocabulary
                .node_kind(instance.kind)
                .is_some_and(|decl| decl.name == "Note")
        })
        .map(|instance| {
            let pitch = match instance.attrs.first().map(|source| source.value) {
                Some(AttrValueSource::Expr(expr, scope)) => reads(engine, expr, scope),
                _ => "no pitch".into(),
            };
            let gate = engine
                .hir
                .placement(instance.placement)
                .gate
                .map(|gate| reads(engine, gate, instance.scope));
            (pitch, gate)
        })
        .collect()
}

#[test]
fn an_index_known_at_compile_time_reads_the_elements_own_binding() {
    let text = "input a = 1\ninput b = 2\ninput j = 0\nlet c = [a, b, a + b]\nlet k = 1\nClip m:\n    Note/(c[1])\n    Note/(c[k])\n    for i in 1..2:\n        Note/(c[i])\n    Note/(c[2])\n    Note/(c[j])\n";
    let (diagnostics, notes) = analyzed(text, note_pitches_and_gates);
    assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
    let pitches: Vec<String> = notes.into_iter().map(|(pitch, _)| pitch).collect();
    assert_eq!(
        pitches,
        [
            "input b[0]",
            "input b[0]",
            "input b[0]",
            "cell c",
            "runtime pick"
        ]
    );
    let text = "input steps = [true, false, false, true, false, false, true, false]\ninput pitches = [36, 36, 43, 36, 41, 36, 43, 48]\n\nClip seq:\n    for i in 0..8:\n        Note/(pitches[i]) when steps[i]\n";
    let (diagnostics, notes) = analyzed(text, note_pitches_and_gates);
    assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
    let expected: Vec<(String, Option<String>)> = (0..8)
        .map(|element| {
            (
                format!("input pitches[{element}]"),
                Some(format!("input steps[{element}]")),
            )
        })
        .collect();
    assert_eq!(notes, expected);
}
