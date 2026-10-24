use symphonyscript_fixtures::{
    Project, ProjectFile, deep_projects, dsp_project, music_project, spec_examples, spec_projects,
    vocabulary,
};

use super::Engine;
use super::attr_source::AttrValueSource;
use symphonyscript_core::{Count, Vocabulary};

use super::binding::BindingKind;
use super::bound::BoundVal;
use super::cval::CVal;
use super::destination::Destination;
use super::eval_ty::EvalTy;
use super::ids::{NodeId, ScopeId};
use super::node::Op;
use super::sig::Kind;
use super::val::Val;
use crate::compiler::Compiler;
use crate::diagnostics::{Code, Diagnostic, Severity};
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
    analyzed_with(&vocabulary(), text, inspect)
}

fn analyzed_with<R>(
    vocabulary: &Vocabulary,
    text: &str,
    inspect: impl FnOnce(&Engine<'_>) -> R,
) -> (Vec<Diagnostic>, R) {
    let compiler = Compiler::new(vocabulary).expect("the vocabulary compiles");
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    compiler.analyzed(&[source], inspect)
}

fn envelope_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("envelope");
    let time = v.unit_family("time").base("s").unit("ms", 0.001);
    let envelope = v.node("Env");
    v.kind(envelope)
        .top_level(Count::Any)
        .attr::<f64>("attack")
        .default(0.0)
        .family(&time, "ms")
        .attr::<f32>("release")
        .default(0.0)
        .family(&time, "ms");
    v.build().expect("the envelope vocabulary builds")
}

const IN_DEFAULT_UNITS: &[&str] = &[
    "Env e | attack=(1s + 3)\n",
    "Env e | attack=(if 1s + 3 > 2s then 1s else 2s)\n",
    "Env e | attack=((1s + 3) / 2)\n",
    "Env e | attack=(1s + 3ms)\n",
];

const FIXED_IN_DEFAULT_UNITS: &[&str] = &[
    "Env e | attack=3i64\n",
    "Env e | attack=i64(3)\n",
    "Env e | attack=(1s + 3i32)\n",
    "Env e | attack=(if 1s + 3i32 > 2s then 1s else 2s)\n",
    "Env e | release=3i32\n",
];

fn first_node(engine: &Engine<'_>) -> Option<NodeId> {
    let instance = engine.instances.last()?;
    let AttrValueSource::Expr(expr, scope) = instance.attrs.first()?.value else {
        return None;
    };
    match engine.memo.exprs.get(&(expr, scope)) {
        Some(Val::Scalar(node)) => Some(*node),
        _ => None,
    }
}

fn first_value_in(engine: &Engine<'_>, ty: EvalTy) -> Option<(CVal, Kind)> {
    let node = first_node(engine)?;
    let value = engine.memo.evals.get(&(node, ty))?.clone();
    Some((value, engine.memo.sigs.get(&node)?.kind))
}

fn first_value(engine: &Engine<'_>) -> Option<(CVal, Kind)> {
    first_value_in(engine, EvalTy::F64)
}

fn compared_in(engine: &Engine<'_>) -> Vec<EvalTy> {
    let Some(node) = first_node(engine) else {
        return Vec::new();
    };
    let Op::If(condition, _, _) = engine.node(node).op else {
        return Vec::new();
    };
    let Op::Binary(_, lhs, _) = engine.node(condition).op else {
        return Vec::new();
    };
    let mut types: Vec<EvalTy> = engine
        .memo
        .evals
        .keys()
        .filter(|(evaluated, _)| *evaluated == lhs)
        .map(|(_, ty)| *ty)
        .collect();
    types.sort_by_key(|ty| format!("{ty:?}"));
    types
}

#[test]
fn a_fixed_type_integer_read_in_a_fractional_unit_is_the_float_it_widens_into() {
    let envelope = envelope_vocabulary();
    let expected = [
        CVal::F64(0.003),
        CVal::F64(0.003),
        CVal::F64(1.0 + 0.003),
        CVal::F64(2.0),
    ];
    for (text, value) in FIXED_IN_DEFAULT_UNITS.iter().zip(expected) {
        let (diagnostics, found) = analyzed_with(&envelope, text, first_value);
        assert_eq!(diagnostics, Vec::<Diagnostic>::new(), "{text}");
        assert_eq!(found.map(|(value, _)| value), Some(value), "{text}");
    }
    let (_, types) = analyzed_with(&envelope, FIXED_IN_DEFAULT_UNITS[3], compared_in);
    assert_eq!(types, vec![EvalTy::F32]);
    let (diagnostics, found) = analyzed_with(&envelope, "Env e | release=3i32\n", |engine| {
        first_value_in(engine, EvalTy::F32)
    });
    assert_eq!(diagnostics, Vec::<Diagnostic>::new());
    assert_eq!(found.map(|(value, _)| value), Some(CVal::F32(0.003)));
    let codes = |vocabulary: &Vocabulary, text: &str| -> Vec<Code> {
        analyzed_with(vocabulary, text, |_| ())
            .0
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
            .map(|diagnostic| diagnostic.code)
            .collect()
    };
    let gain = codes(&vocabulary(), "Clip c | gain=3i64\n");
    assert_eq!(gain, vec![Code::TypeMismatch]);
    assert_eq!(codes(&envelope, "Env e | release=3i64\n"), gain);
}

#[test]
fn a_plain_value_reads_in_the_default_unit_as_a_literal_with_its_suffix_would() {
    let envelope = envelope_vocabulary();
    let expected = [
        (1.0 + 0.003, Kind::Float),
        (2.0, Kind::Int),
        ((1.0 + 0.003) / 2.0, Kind::Float),
        (1.0 + 0.003, Kind::Float),
    ];
    for (text, (value, kind)) in IN_DEFAULT_UNITS.iter().zip(expected) {
        let (diagnostics, found) = analyzed_with(&envelope, text, first_value);
        assert_eq!(diagnostics, Vec::<Diagnostic>::new(), "{text}");
        assert_eq!(found, Some((CVal::F64(value), kind)), "{text}");
    }
    let (diagnostics, _) = analyzed_with(&envelope, "Env e | attack=((1s + 3) div 2)\n", |_| ());
    let codes: Vec<Code> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, vec![Code::OperandType]);
    let (diagnostics, found) = analyzed("Clip c:\n    Note | length=2bars + 1\n", first_value);
    assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
    assert_eq!(found, Some((CVal::F64(9.0), Kind::Int)));
}

fn bar_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("bars");
    let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
    let bar = v.node("Bar");
    v.kind(bar)
        .top_level(Count::Any)
        .attr::<i64>("len")
        .default(0)
        .family(&beats, "bars")
        .attr::<i64>("beat_len")
        .default(0)
        .family(&beats, "beats")
        .attr::<i64>("plain")
        .default(0);
    v.build().expect("the bar vocabulary builds")
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

fn error_codes(diagnostics: &[Diagnostic]) -> Vec<Code> {
    errors_of(diagnostics)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn compile_time_values_follow_the_types_that_flow_into_them() {
    let example = vocabulary();
    let bars = bar_vocabulary();
    let fib = "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)\n";
    let r = "fn r(n): if n < 1 then 1bars else r(n - 1) + 1\n";
    let q = "fn q(n): if n < 1 then 1bars else q(n - 1)\n";
    let rows: Vec<(&Vocabulary, String, &str, EvalTy, CVal)> = vec![
        (
            &example,
            "Clip c | gain=((16777216 + 1) / 2)\n".into(),
            "gain",
            EvalTy::F32,
            CVal::F32(8388608.0),
        ),
        (
            &example,
            "Clip c | gain=(16777216 + 1)\n".into(),
            "gain",
            EvalTy::F32,
            CVal::F32(16777216.0),
        ),
        (
            &example,
            "Osc o | freq=(2 ** -1 / 1)\n".into(),
            "freq",
            EvalTy::F64,
            CVal::F64(0.5),
        ),
        (
            &example,
            "Osc o | freq=((16777216 + 1) / 2)\n".into(),
            "freq",
            EvalTy::F64,
            CVal::F64(8388608.5),
        ),
        (
            &example,
            "fn s(n): if n < 1 then 0 else (n div 2) + s(n - 1)\nOsc o | freq=(s(7))\n".into(),
            "freq",
            EvalTy::F64,
            CVal::F64(12.0),
        ),
        (
            &example,
            "fn g(n): if n < 1 then 0 else [10, 20, 30][n] + g(n - 1)\nOsc o | freq=(g(2))\n"
                .into(),
            "freq",
            EvalTy::F64,
            CVal::F64(50.0),
        ),
        (
            &example,
            format!("{fib}Clip c:\n    Note | length=(fib(3))\n"),
            "length",
            EvalTy::F64,
            CVal::F64(2.0),
        ),
        (
            &bars,
            format!("{q}Bar b | len=(q(0))\n"),
            "len",
            EvalTy::I64,
            CVal::Int(4),
        ),
        (
            &bars,
            format!("{r}Bar b | len=(r(2))\n"),
            "len",
            EvalTy::I64,
            CVal::Int(12),
        ),
        (
            &bars,
            format!("{r}Bar b | beat_len=(r(2))\n"),
            "beat_len",
            EvalTy::I64,
            CVal::Int(6),
        ),
        (
            &bars,
            format!("{r}Bar b | len=(r(2)) beat_len=(r(2))\n"),
            "len",
            EvalTy::I64,
            CVal::Int(12),
        ),
        (
            &bars,
            format!("{r}Bar b | len=(r(2)) beat_len=(r(2))\n"),
            "beat_len",
            EvalTy::I64,
            CVal::Int(6),
        ),
        (
            &bars,
            format!("{r}Bar b | len=(r(0) + 1)\n"),
            "len",
            EvalTy::I64,
            CVal::Int(8),
        ),
        (
            &bars,
            "Bar b | len=(1bars + 1)\n".into(),
            "len",
            EvalTy::I64,
            CVal::Int(8),
        ),
        (
            &example,
            "Clip c | gain=(rand(0, 0))\n".into(),
            "gain",
            EvalTy::F32,
            CVal::F32(14819496.0 / 16777216.0),
        ),
        (
            &example,
            "Osc o | freq=(max(0.0 / 0.0, 440.0))\n".into(),
            "freq",
            EvalTy::F64,
            CVal::F64(440.0),
        ),
        (
            &example,
            "Clip c:\n    Note/((-(2 ** 126) - (2 ** 126 - 1)) div 2 ** 120)\n".into(),
            "pitch",
            EvalTy::I32,
            CVal::Int(-128),
        ),
    ];
    let mut failures = Vec::new();
    for (vocabulary, text, attr, ty, expected) in rows {
        let (diagnostics, found) =
            analyzed_with(vocabulary, &text, |engine| attr_value(engine, attr, ty));
        let errors = error_codes(&diagnostics);
        if !errors.is_empty() || found.as_ref() != Some(&expected) {
            failures.push(format!(
                "{text:?} {attr}: expected {expected:?}, found {found:?} with {errors:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_recursive_call_repeats_an_earlier_one_only_with_an_equal_expected_type() {
    let fib = "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)\n";
    let (diagnostics, one) = analyzed(&format!("{fib}Clip c:\n    Note/(fib(3))\n"), |engine| {
        engine.call_count
    });
    assert_eq!(error_codes(&diagnostics), Vec::<Code>::new());
    let (diagnostics, both) = analyzed(
        &format!("{fib}Clip c:\n    Note/(fib(3))\nOsc o | freq=(fib(3))\n"),
        |engine| engine.call_count,
    );
    assert_eq!(error_codes(&diagnostics), Vec::<Code>::new());
    assert_eq!((one, both), (4, 5));
}

#[test]
fn unit_rules_cover_recursive_results_and_bodies() {
    let bars = bar_vocabulary();
    let q = "fn q(n): if n < 1 then 1bars else q(n - 1)\n";
    let rows: Vec<(String, Vec<Code>)> = vec![
        (
            format!("{q}Bar b | plain=(q(0))\n"),
            vec![Code::UnitMismatch],
        ),
        (format!("{q}Bar b | len=(q(0))\n"), vec![]),
        ("let d = 2bars + 1\nBar b | len=d\n".into(), vec![]),
        (
            "let d = 2bars + 1\nBar b | len=d beat_len=d\n".into(),
            vec![Code::ConflictingUnits],
        ),
        (
            "Bar b | len=42535295865117307932921825928971026432\n".into(),
            vec![Code::IntegerOverflow],
        ),
        (
            "Bar b | len=(42535295865117307932921825928971026432 + 0)\n".into(),
            vec![Code::IntegerOverflow],
        ),
        (
            "Bar b | len=42535295865117307932921825928971026431\n".into(),
            vec![Code::DoesNotFit],
        ),
        (
            "Bar b | len=(-42535295865117307932921825928971026432 + 0)\n".into(),
            vec![Code::IntegerOverflow],
        ),
        (
            "Bar b | len=(-42535295865117307932921825928971026431 + 0)\n".into(),
            vec![Code::DoesNotFit],
        ),
    ];
    let mut failures = Vec::new();
    for (text, expected) in rows {
        let (diagnostics, ()) = analyzed_with(&bars, &text, |_| ());
        let found = error_codes(&diagnostics);
        if found != expected {
            failures.push(format!("{text:?}: expected {expected:?}, found {found:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
    let envelope = envelope_vocabulary();
    for text in IN_DEFAULT_UNITS.iter().chain(FIXED_IN_DEFAULT_UNITS) {
        let (diagnostics, found) = analyzed_with(&envelope, text, placeholders);
        assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new(), "{text}");
        assert_eq!(found, Vec::<String>::new(), "{text}");
    }
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

fn note_conditions(engine: &Engine<'_>, text: &str) -> Vec<Vec<(String, bool)>> {
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
            engine
                .conditions_of(instance.conditions)
                .into_iter()
                .map(|condition| {
                    let loc = engine.hir.expr(condition.expr).loc;
                    let guard = text
                        .get(loc.start as usize..loc.end as usize)
                        .unwrap_or_default()
                        .to_string();
                    (guard, condition.holds)
                })
                .collect()
        })
        .collect()
}

#[test]
fn an_arm_is_active_when_its_guard_holds_and_no_earlier_guard_does() {
    let text = "input fill_on = false\ninput section = 0\nClip verse:\n    Note/60\n    when fill_on:\n        Note/62 Note/64\n    else:\n        Note/62\n    when:\n        section is 0: Note/67\n        section is 1: Note/69\n        else: Note/71\n";
    let (diagnostics, found) = analyzed(text, |engine| note_conditions(engine, text));
    assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
    let held = |guard: &str, holds: bool| (guard.to_string(), holds);
    assert_eq!(
        found,
        vec![
            vec![],
            vec![held("fill_on", true)],
            vec![held("fill_on", true)],
            vec![held("fill_on", false)],
            vec![held("section is 0", true)],
            vec![held("section is 0", false), held("section is 1", true)],
            vec![held("section is 0", false), held("section is 1", false)],
        ]
    );
    let deep = deep_projects()
        .into_iter()
        .find(|project| project.name == "deep bodies")
        .expect("the deep bodies project exists");
    let deep_text = deep
        .files
        .first()
        .map(|file| file.text.clone())
        .unwrap_or_default();
    let (_, conditions) = std::thread::scope(|threads| {
        std::thread::Builder::new()
            .stack_size(1 << 20)
            .spawn_scoped(threads, || {
                analyzed(&deep_text, |engine| {
                    let notes = note_conditions(engine, &deep_text);
                    let last = notes.last().cloned().unwrap_or_default();
                    let links = engine.condition_links.len();
                    (last, links)
                })
            })
            .expect("the thread starts")
            .join()
            .expect("the deep bodies analyze")
    });
    let (last, links) = conditions;
    assert_eq!(last.len(), 2000);
    assert!(last.iter().all(|(guard, holds)| guard == "x" && *holds));
    assert!(links <= 4000, "{links} condition links");
}

#[test]
fn guard_arms_cost_linear_time_and_memory() {
    let size = |arms: usize| {
        let mut text = String::from("input x = 0\nClip c:\n    when:\n");
        for arm in 0..arms {
            text.push_str(&format!("        x is {arm}: Note/{arm}\n"));
        }
        let (diagnostics, size) = analyzed(&text, |engine| {
            engine.nodes.len() + engine.condition_links.len() + engine.gates.len()
        });
        assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
        size
    };
    let sizes: Vec<usize> = [1000, 2000, 4000, 8000].into_iter().map(size).collect();
    for pair in sizes.windows(2) {
        if let [smaller, larger] = pair {
            assert!(
                *larger * 10 <= *smaller * 22,
                "{sizes:?} grows faster than 2.2 per doubling"
            );
        }
    }
}
