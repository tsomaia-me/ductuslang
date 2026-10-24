use symphonyscript_core::{Count, Scalar, Vocabulary};
use symphonyscript_fixtures::{
    Project, ProjectFile, deep_projects, dsp_project, music_project, spec_examples, spec_projects,
    vocabulary,
};

use super::Engine;
use super::attr_value_source::AttrValueSource;
use super::binding_kind::BindingKind;
use super::bound_val::BoundVal;
use super::c_array::CArray;
use super::c_val::CVal;
use super::destination::Destination;
use super::eval_ty::EvalTy;
use super::ids::{BindingId, NodeId, ScopeId};
use super::kind::Kind;
use super::op::Op;
use super::val::Val;
use crate::compiler::Compiler;
use crate::diagnostics::{Code, Diagnostic, Severity};
use crate::semantics::collection::{Variants, collect};
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
    let variants = Variants::of(&vocabulary);
    let collected = collect(&hir, &vocabulary, &variants, &mut report);
    let mut engine = Engine::new(&hir, &vocabulary, &collected, &variants);
    engine.expand();
    engine.infer();
    engine.check_program();
    engine.flush_diagnostics();
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
    "Env e | attack=((1s + 3) / 2)\n",
    "Env e | attack=(1s + 3ms)\n",
];

const FIXED_IN_DEFAULT_UNITS: &[&str] = &[
    "Env e | attack=3i64\n",
    "Env e | attack=i64(3)\n",
    "Env e | attack=(1s + 3i32)\n",
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

#[test]
fn a_fixed_type_integer_read_in_a_fractional_unit_is_the_float_it_widens_into() {
    let envelope = envelope_vocabulary();
    let expected = [CVal::F64(0.003), CVal::F64(0.003), CVal::F64(1.0 + 0.003)];
    for (text, value) in FIXED_IN_DEFAULT_UNITS.iter().zip(expected) {
        let (diagnostics, found) = analyzed_with(&envelope, text, first_value);
        assert_eq!(diagnostics, Vec::<Diagnostic>::new(), "{text}");
        assert_eq!(found.map(|(value, _)| value), Some(value), "{text}");
    }
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

fn attr_values(engine: &Engine<'_>, name: &str, ty: EvalTy) -> Vec<CVal> {
    let mut values = Vec::new();
    for instance in &engine.instances {
        let Some(decl) = engine.vocabulary.node_kind(instance.kind) else {
            continue;
        };
        for source in &instance.attrs {
            let named = decl
                .attrs
                .get(source.attr.index())
                .is_some_and(|attr| attr.name == name);
            if let (true, AttrValueSource::Expr(expr, scope)) = (named, source.value)
                && let Some(Val::Scalar(node)) = engine.memo.exprs.get(&(expr, scope))
                && let Some(value) = engine.memo.evals.get(&(*node, ty))
            {
                values.push(value.clone());
            }
        }
    }
    values
}

#[test]
fn a_plain_value_reads_in_the_unit_its_family_value_reaches() {
    let example = vocabulary();
    let bars = bar_vocabulary();
    let family = "2bars + 1";
    let mut rejected: Vec<(&Vocabulary, String)> = vec![
        (
            &example,
            format!("Clip c:\n    Note | length=(({family}) div 1bars)\n"),
        ),
        (
            &example,
            format!("Clip c:\n    for i in 0..(({family}) div 1bars):\n        Note\n"),
        ),
        (
            &bars,
            format!("let k = ({family}) div 1bars\nBar b | len=k\n"),
        ),
        (
            &example,
            format!("Clip c:\n    Note when {family} > 3bars\n"),
        ),
    ];
    let plain = [
        format!("({family}) div 1bars"),
        format!("({family}) / 1bars"),
    ];
    let plain_uses = [
        "Clip c:\n    Note | length=({})\n",
        "let k = {}\nClip c:\n    Note | length=k\n",
        "Clip c:\n    Note when ({}) > 1\n",
        "Clip c | gain=({})\n",
    ];
    for wrapped in &plain {
        for user in plain_uses {
            rejected.push((&example, user.replace("{}", wrapped)));
        }
    }
    rejected.push((
        &example,
        format!("Clip c:\n    for i in 0..({}):\n        Note\n", plain[0]),
    ));
    for wrapped in [format!("{family} > 3bars"), format!("{family} <= 3bars")] {
        for user in [
            "Clip c:\n    Note when {}\n",
            "Clip c:\n    Note | accent=({})\n",
        ] {
            rejected.push((&example, user.replace("{}", &wrapped)));
        }
    }
    let mut failures = Vec::new();
    for (vocabulary, text) in &rejected {
        let (diagnostics, ()) = analyzed_with(vocabulary, text, |_| ());
        let start = text.find(family).map(|at| at + family.len() - 1);
        let found: Vec<(Code, Option<usize>)> = errors_of(&diagnostics)
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.code,
                    usize::try_from(diagnostic.span.range.start).ok(),
                )
            })
            .collect();
        if found != [(Code::PlainInFamily, start)] {
            failures.push(format!("{text:?}: found {found:?}"));
        }
    }
    let envelope = envelope_vocabulary();
    for (vocabulary, text, plain) in [
        (
            &envelope,
            "Env e | attack=(if 1s + 3 > 2s then 1s else 2s)\n",
            "3 >",
        ),
        (
            &envelope,
            "Env e | attack=(if 1s + 3i32 > 2s then 1s else 2s)\n",
            "3i32",
        ),
        (
            &example,
            "fn u(n): if n < 1 then (n * 1bars + 1) div 1bars else u(n - 1)\nClip c:\n    Note | length=(u(2))\n",
            "1) div",
        ),
    ] {
        let (diagnostics, ()) = analyzed_with(vocabulary, text, |_| ());
        let found: Vec<(Code, Option<u32>)> = errors_of(&diagnostics)
            .iter()
            .map(|diagnostic| (diagnostic.code, Some(diagnostic.span.range.start)))
            .collect();
        let start = text.find(plain).and_then(|at| u32::try_from(at).ok());
        if found != [(Code::PlainInFamily, start)] {
            failures.push(format!("{text:?}: found {found:?}"));
        }
    }
    let mut accepted: Vec<(String, Vec<f64>)> = vec![
        (format!("Clip c:\n    Note | length={family}\n"), vec![9.0]),
        (
            "Clip c:\n    Note | length=((2bars + 4bars) div 1bars)\n".into(),
            vec![6.0],
        ),
        (
            format!("Clip c:\n    for x in [1bars, {family}]:\n        Note | length=x\n"),
            vec![4.0, 9.0],
        ),
        (
            format!("let d = {family}\nClip c:\n    Note | length=d\n"),
            vec![9.0],
        ),
    ];
    for (wrapped, value) in [
        (format!("({family}) * 2"), 18.0),
        (format!("({family}) / 2"), 4.5),
        (format!("-({family})"), -9.0),
        (format!("abs({family})"), 9.0),
        (format!("min({family}, 3bars)"), 9.0),
        (format!("if true then {family} else 1bars"), 9.0),
    ] {
        accepted.push((
            format!("Clip c:\n    Note | length=({wrapped})\n"),
            vec![value],
        ));
    }
    for (text, expected) in &accepted {
        let (diagnostics, found) =
            analyzed(text, |engine| attr_values(engine, "length", EvalTy::F64));
        let expected: Vec<CVal> = expected.iter().map(|value| CVal::F64(*value)).collect();
        if !errors_of(&diagnostics).is_empty() || found != expected {
            failures.push(format!(
                "{text:?}: expected {expected:?}, found {found:?} with {:?}",
                error_codes(&diagnostics)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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
    let recursive_arrays = [
        format!("{CHORD_FN}Clip c:\n    [Note/(chord(3)[0]) Note/(chord(3)[1]) Note/(chord(3)[2])]\n"),
        format!("{R_FN}Clip c | gain=(r(0.1, 3)[0])\nOsc o | freq=(r(0.1, 3)[0])\n"),
        format!("{CHORD_FN}Clip c:\n    for p in chord(3):\n        Note/p\n"),
        format!("{CHORD_FN}let v: i32[3] = chord(1)\nlet x = [chord(1), chord(2)]\nClip c:\n    Note/(v[2] + x[1][2])\n"),
        format!("{CHORD_FN}input f = true\ninput a = 1\ninput i = 0\nClip c:\n    Note/((if f then chord(1) else [1, 2, 3])[0])\n    Note/([a, chord(3)[0]][1])\n    Note/(chord(3)[i])\n"),
        "fn g(k): if k < 1 then [[1, 2, 3], [4, 5, 6]] else g(k - 1)\nfn rev(a, k): if k < 1 then a else rev([a[1], a[0]], k - 1)\nfn grow(k): if k < 1 then [0] else [grow(k - 1)[0] + 1, 0]\nClip c:\n    Note/(g(2)[1][2] + rev([1, 2], 3)[0] + grow(3)[0])\n".into(),
    ];
    for text in &recursive_arrays {
        let (diagnostics, found) = analyzed(text, placeholders);
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

#[test]
fn equal_sibling_calls_share_one_inlined_body() {
    let work = |levels: usize| {
        let mut text = String::from("input i = 1\nfn f0(x): x\n");
        for level in 1..=levels {
            let below = level - 1;
            text.push_str(&format!("fn f{level}(x): f{below}(x) + f{below}(x)\n"));
        }
        text.push_str(&format!("Clip c:\n    Note/(f{levels}(i))\n"));
        let (diagnostics, work) = analyzed(&text, |engine| {
            (engine.nodes.len(), engine.scopes.len(), engine.inlined)
        });
        assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
        work
    };
    let (six, twelve, eighteen) = (work(6), work(12), work(18));
    assert_eq!(eighteen.0 - twelve.0, twelve.0 - six.0, "nodes");
    assert_eq!(eighteen.1 - twelve.1, twelve.1 - six.1, "scopes");
    assert_eq!(eighteen.2, (1 << 19) - 1, "inlined calls");
}

#[test]
fn equal_calls_in_different_positions_are_inlined_separately() {
    let bars = bar_vocabulary();
    let g = "fn g(): 1bars + 1\n";
    for text in [
        format!("{g}Bar b | len=(g()) beat_len=(g())\n"),
        format!("{g}let x = g()\nlet y = g()\nBar b | len=x beat_len=y\n"),
    ] {
        let (diagnostics, found) = analyzed_with(&bars, &text, |engine| {
            (
                attr_value(engine, "len", EvalTy::I64),
                attr_value(engine, "beat_len", EvalTy::I64),
            )
        });
        assert_eq!(error_codes(&diagnostics), Vec::<Code>::new(), "{text}");
        assert_eq!(found, (Some(CVal::Int(8)), Some(CVal::Int(5))), "{text}");
    }
}

const R_FN: &str =
    "fn r(x, n): if n < 1 then (if x + 0.2 is 0.3 then [x] else [x, x]) else r(x, n - 1)\n";

const CHORD_FN: &str =
    "fn chord(k): if k is 0 then [60, 64, 67] else if k is 1 then [62, 65, 69] else chord(k - 2)\n";

fn recursive_results(engine: &Engine<'_>) -> Vec<(EvalTy, CVal)> {
    let mut found: Vec<(EvalTy, CVal)> = engine
        .memo
        .evals
        .iter()
        .filter(|((node, _), _)| matches!(engine.node(*node).op, Op::Recursive { .. }))
        .map(|((_, ty), value)| (*ty, value.clone()))
        .collect();
    found.sort_by_key(|(ty, _)| format!("{ty:?}"));
    found
}

fn located(text: &str, diagnostics: &[Diagnostic]) -> Vec<(Code, usize, Vec<usize>)> {
    errors_of(diagnostics)
        .iter()
        .map(|diagnostic| {
            let at = |start: u32| usize::try_from(start).unwrap_or(usize::MAX);
            (
                diagnostic.code,
                at(diagnostic.span.range.start),
                diagnostic
                    .secondary
                    .iter()
                    .map(|span| at(span.range.start))
                    .filter(|start| *start < text.len())
                    .collect(),
            )
        })
        .collect()
}

#[test]
fn a_recursive_array_result_takes_the_shape_it_yields_in_the_type_its_use_expects() {
    let mut failures = Vec::new();
    let shared = format!("{R_FN}let v = r(0.1, 3)\nClip c | gain=(v[0])\nOsc o | freq=(v[0])\n");
    let (diagnostics, ()) = analyzed(&shared, |_| ());
    let first = shared.find("v[0]").unwrap_or(0);
    let second = shared.rfind("v[0]").unwrap_or(0);
    let found = located(&shared, &diagnostics);
    if found != [(Code::InconsistentShapes, first, vec![second])] {
        failures.push(format!("{shared:?}: found {found:?}"));
    }
    let uses = [
        ("Clip a | gain=({})\n", true),
        ("Clip b | level=({})\n", true),
        ("Osc d | rate=({})\n", true),
        ("Osc e | freq=({})\n", false),
        ("Session s | tempo=({})\n", false),
    ];
    for (one, one_f32) in uses {
        for (other, other_f32) in uses {
            if one == other {
                continue;
            }
            let held = format!(
                "{R_FN}let v = r(0.1, 3)\n{}{}",
                one.replace("{}", "v[0]"),
                other.replace("{}", "v[0]")
            );
            let expected = if one_f32 == other_f32 {
                Vec::new()
            } else {
                vec![Code::InconsistentShapes]
            };
            let (diagnostics, ()) = analyzed(&held, |_| ());
            if error_codes(&diagnostics) != expected {
                failures.push(format!("{held:?}: found {:?}", error_codes(&diagnostics)));
            }
            let separate = format!(
                "{R_FN}{}{}",
                one.replace("{}", "r(0.1, 3)[0]"),
                other.replace("{}", "r(0.1, 3)[0]")
            );
            let (diagnostics, ()) = analyzed(&separate, |_| ());
            if !errors_of(&diagnostics).is_empty() {
                failures.push(format!(
                    "{separate:?}: found {:?}",
                    error_codes(&diagnostics)
                ));
            }
        }
    }
    let separate = format!("{R_FN}Clip c | gain=(r(0.1, 3)[0])\nOsc o | freq=(r(0.1, 3)[0])\n");
    let (diagnostics, (results, gain, freq)) = analyzed(&separate, |engine| {
        (
            recursive_results(engine),
            attr_values(engine, "gain", EvalTy::F32),
            attr_values(engine, "freq", EvalTy::F64),
        )
    });
    let tenth = 0.1;
    let expected = vec![
        (EvalTy::F32, CVal::Array(CArray::new(vec![CVal::F32(0.1)]))),
        (
            EvalTy::F64,
            CVal::Array(CArray::new(vec![CVal::F64(tenth), CVal::F64(tenth)])),
        ),
    ];
    if !errors_of(&diagnostics).is_empty()
        || results != expected
        || gain != [CVal::F32(0.1)]
        || freq != [CVal::F64(tenth)]
    {
        failures.push(format!(
            "{separate:?}: {results:?}, {gain:?}, {freq:?} with {:?}",
            error_codes(&diagnostics)
        ));
    }
    let chords = [[60, 64, 67], [62, 65, 69]];
    for depth in 0..6usize {
        let mut text = format!("{CHORD_FN}Clip c:\n    [");
        for position in 0..3 {
            text.push_str(&format!("Note/(chord({depth})[{position}]) "));
        }
        text.push_str("]\n");
        let (diagnostics, pitches) =
            analyzed(&text, |engine| attr_values(engine, "pitch", EvalTy::I32));
        let expected: Vec<CVal> = chords
            .get(depth % 2)
            .map(|chord| chord.iter().map(|pitch| CVal::Int(*pitch)).collect())
            .unwrap_or_default();
        if !errors_of(&diagnostics).is_empty() || pitches != expected {
            failures.push(format!(
                "{text:?}: {pitches:?} with {:?}",
                error_codes(&diagnostics)
            ));
        }
        let past = format!("{CHORD_FN}Clip c:\n    Note/(chord({depth})[3])\n");
        let (diagnostics, ()) = analyzed(&past, |_| ());
        let index = past.rfind("[3]").map_or(0, |at| at + 1);
        let found = located(&past, &diagnostics);
        if found != [(Code::IndexOutOfRange, index, Vec::new())] {
            failures.push(format!("{past:?}: found {found:?}"));
        }
    }
    let grid = "fn g(k): if k < 1 then [[1, 2, 3], [4, 5, 6]] else g(k - 1)\n";
    for row in 0..2 {
        for column in 0..3 {
            let text = format!("{grid}Clip c:\n    Note/(g(2)[{row}][{column}])\n");
            let (diagnostics, pitches) =
                analyzed(&text, |engine| attr_values(engine, "pitch", EvalTy::I32));
            let expected = [CVal::Int(1 + 3 * row + column)];
            if !errors_of(&diagnostics).is_empty() || pitches != expected {
                failures.push(format!("{text:?}: {pitches:?}"));
            }
        }
    }
    let rows: Vec<(String, Vec<Code>, Vec<i128>)> = vec![
        (
            format!("{grid}Clip c:\n    Note/(g(2)[0][3])\n"),
            vec![Code::IndexOutOfRange],
            vec![],
        ),
        (
            format!("{grid}Clip c:\n    Note/(g(2)[2][0])\n"),
            vec![Code::IndexOutOfRange],
            vec![],
        ),
        (
            format!("{CHORD_FN}Clip c:\n    Note/(chord(1))\n"),
            vec![Code::TypeMismatch],
            vec![],
        ),
        (
            format!("{CHORD_FN}Clip c:\n    Note/(chord(1) + 1)\n"),
            vec![Code::ArrayOperand],
            vec![],
        ),
        (
            format!("{CHORD_FN}input a = 1\nClip c:\n    Note/(a + chord(1))\n"),
            vec![Code::ArrayOperand],
            vec![],
        ),
        (
            format!("{CHORD_FN}Clip c:\n    Note/(abs(chord(1)))\n"),
            vec![Code::ArrayOperand],
            vec![],
        ),
        (
            format!("{CHORD_FN}Clip c:\n    Note/([1, 2][chord(1)])\n"),
            vec![Code::IndexType],
            vec![],
        ),
        (
            format!("{CHORD_FN}Kit k:\n    Sampler | file=\"{{chord(1)}}\"\n"),
            vec![Code::InterpolationType],
            vec![],
        ),
        (
            "fn b(k): if k < 1 then [true, false] else b(k - 1)\nClip c:\n    Note when b(1)\n"
                .into(),
            vec![Code::ConditionType],
            vec![],
        ),
        (
            "fn b(k): if k < 1 then [true, false] else b(k - 1)\nClip c:\n    Note/60 when b(1)[0]\n"
                .into(),
            vec![],
            vec![60],
        ),
        (
            "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)\nClip c:\n    Note/(fib(3)[0])\n"
                .into(),
            vec![Code::NotAnArray],
            vec![],
        ),
        (
            format!("{CHORD_FN}let v: i32[3] = chord(1)\nClip c:\n    Note/(v[2])\n"),
            vec![],
            vec![69],
        ),
        (
            format!("{CHORD_FN}let v: i32[2] = chord(1)\nClip c:\n    Note/(v[1])\n"),
            vec![Code::TypeMismatch],
            vec![],
        ),
        (
            format!("{CHORD_FN}Clip c:\n    for p in chord(3):\n        Note/p\n"),
            vec![],
            vec![62, 65, 69],
        ),
        (
            format!("{CHORD_FN}let x = [chord(1), chord(2)]\nClip c:\n    Note/(x[1][2])\n"),
            vec![],
            vec![67],
        ),
        (
            format!("{CHORD_FN}input f = true\nClip c:\n    Note/((if f then chord(1) else [1, 2, 3])[0])\n"),
            vec![],
            vec![],
        ),
        (
            format!("{CHORD_FN}input f = true\nClip c:\n    Note/((if f then chord(1) else [1, 2])[0])\n"),
            vec![Code::TypeMismatch],
            vec![],
        ),
        (
            format!("{CHORD_FN}input a = 1\nClip c:\n    Note/([a, chord(3)[0]][1])\n"),
            vec![],
            vec![62],
        ),
        (
            format!("{CHORD_FN}input i = 0\nClip c:\n    Note/(chord(3)[i])\n"),
            vec![],
            vec![],
        ),
        (
            format!("{CHORD_FN}input f = true\nlet x = if f then chord(1) else chord(2)\nClip c:\n    Note/(x[0])\n"),
            vec![Code::TypeMismatch],
            vec![],
        ),
        (
            "fn rev(a, k): if k < 1 then a else rev([a[1], a[0]], k - 1)\nClip c:\n    Note/(rev([1, 2], 3)[0])\n"
                .into(),
            vec![],
            vec![2],
        ),
        (
            "fn grow(k): if k < 1 then [0] else [grow(k - 1)[0] + 1, 0]\nClip c:\n    Note/(grow(3)[0])\n"
                .into(),
            vec![],
            vec![3],
        ),
        (
            "fn pair(k): if k < 1 then [1, 2] else [pair(k - 1)[1], 3]\nClip c:\n    Note/(pair(1)[0])\n    Note/(pair(2)[0])\n"
                .into(),
            vec![],
            vec![2, 3],
        ),
    ];
    for (text, codes, pitches) in rows {
        let (diagnostics, found) =
            analyzed(&text, |engine| attr_values(engine, "pitch", EvalTy::I32));
        let found: Vec<i128> = found
            .iter()
            .filter_map(|value| match value {
                CVal::Int(pitch) => Some(*pitch),
                _ => None,
            })
            .collect();
        if error_codes(&diagnostics) != codes || (codes.is_empty() && found != pitches) {
            failures.push(format!(
                "{text:?}: {found:?} with {:?}",
                error_codes(&diagnostics)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn chord_vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("chords");
    let chord = v.node("Chord");
    v.kind(chord)
        .top_level(Count::Any)
        .attr::<[i32; 3]>("notes")
        .default([0, 0, 0])
        .attr::<[[i32; 3]; 2]>("grid")
        .default([[0, 0, 0], [0, 0, 0]]);
    v.build().expect("the chord vocabulary builds")
}

#[test]
fn a_recursive_array_result_fills_an_array_attribute_or_input_of_its_shape() {
    let chords = chord_vocabulary();
    let grid = "fn g(k): if k < 1 then [[1, 2, 3], [4, 5, 6]] else g(k - 1)\n";
    let row = |values: &[i128]| {
        CVal::Array(CArray::new(
            values.iter().map(|value| CVal::Int(*value)).collect(),
        ))
    };
    let rows: Vec<(String, &str, Vec<Code>, Option<CVal>)> = vec![
        (
            format!("{CHORD_FN}Chord a | notes=(chord(1))\n"),
            "notes",
            vec![],
            Some(row(&[62, 65, 69])),
        ),
        (
            format!("{CHORD_FN}Chord a | notes=(chord(1)[0])\n"),
            "notes",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            format!(
                "{CHORD_FN}input f = true\nChord a | notes=(if f then chord(1) else chord(2))\n"
            ),
            "notes",
            vec![],
            None,
        ),
        (
            format!("{CHORD_FN}input f = true\nChord a | notes=(if f then chord(1) else [1, 2])\n"),
            "notes",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            format!("{grid}Chord a | grid=(g(1))\n"),
            "grid",
            vec![],
            Some(CVal::Array(CArray::new(vec![
                row(&[1, 2, 3]),
                row(&[4, 5, 6]),
            ]))),
        ),
        (
            format!("{grid}Chord a | grid=(g(1)[1])\n"),
            "grid",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            format!(
                "{CHORD_FN}input f = true\nChord a | notes=(if f then chord(1) else chord(2)[0])\n"
            ),
            "notes",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            format!("{grid}input i = 0\nChord a | notes=(g(1)[i])\n"),
            "notes",
            vec![],
            None,
        ),
        (
            format!("{grid}input i = 0\nChord a | notes=(g(1)[i][0])\n"),
            "notes",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            format!("{grid}input f = true\nChord a | grid=(if f then g(1) else g(2))\n"),
            "grid",
            vec![],
            None,
        ),
    ];
    let mut failures = Vec::new();
    for (text, attr, codes, value) in rows {
        let (diagnostics, (found, holes)) = analyzed_with(&chords, &text, |engine| {
            (attr_values(engine, attr, EvalTy::I32), placeholders(engine))
        });
        let found = found.into_iter().next();
        let clean = !codes.is_empty() || holes.is_empty();
        if error_codes(&diagnostics) != codes || (value.is_some() && found != value) || !clean {
            failures.push(format!(
                "{text:?}: {found:?} with {:?}",
                error_codes(&diagnostics)
            ));
        }
    }
    let annotated = format!("{CHORD_FN}input v: i32[3] = chord(1)\nClip c:\n    Note/(v[2])\n");
    let (diagnostics, holes) = analyzed(&annotated, placeholders);
    if !errors_of(&diagnostics).is_empty() || !holes.is_empty() {
        failures.push(format!(
            "{annotated:?}: {holes:?} with {:?}",
            error_codes(&diagnostics)
        ));
    }
    let wrong = format!("{CHORD_FN}input v: i32[2] = chord(1)\nClip c:\n    Note/(v[1])\n");
    let (diagnostics, ()) = analyzed(&wrong, |_| ());
    if error_codes(&diagnostics) != [Code::TypeMismatch] {
        failures.push(format!("{wrong:?}: {:?}", error_codes(&diagnostics)));
    }
    let input = format!("{CHORD_FN}input v = chord(1)\nClip c:\n    Note/(v[2])\n");
    let (_, slots) = analyzed(&input, |engine| {
        engine
            .memo
            .inputs
            .values()
            .map(|bound| match bound {
                BoundVal::Array(array) => array.1.len(),
                BoundVal::Scalar(_) | BoundVal::Error => 1,
            })
            .collect::<Vec<usize>>()
    });
    if slots != [3] {
        failures.push(format!("{input:?}: input slots {slots:?}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

const FIB_FN: &str = "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)\n";

fn notes_and_pitches(engine: &Engine<'_>) -> (usize, Vec<CVal>) {
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
    (notes, attr_values(engine, "pitch", EvalTy::I32))
}

fn cell_type_and_value(engine: &Engine<'_>, name: &str) -> Option<(Scalar, CVal)> {
    let position = engine
        .bindings
        .iter()
        .position(|binding| binding.name == name)?;
    let binding = BindingId(u32::try_from(position).ok()?);
    let own = *engine.typing.own.get(&binding)?;
    let ty = match own {
        Scalar::I32 => EvalTy::I32,
        Scalar::I64 => EvalTy::I64,
        Scalar::F32 => EvalTy::F32,
        Scalar::F64 => EvalTy::F64,
        Scalar::Bool | Scalar::Enum(_) | Scalar::Str => return None,
    };
    let init = engine.binding(binding).init;
    let value = engine.memo.evals.get(&(init, ty)).cloned()?;
    Some((own, value))
}

#[test]
fn a_recursive_result_has_the_kind_its_body_gives_it() {
    let arp = "pattern Arp(default root, steps = 4, gap = 4):\n    for i in 0..steps:\n        Note/(root + i * gap)\n";
    let pitches = |values: &[i128]| values.iter().map(|value| CVal::Int(*value)).collect();
    let structural: Vec<(String, usize, Vec<CVal>)> = vec![
        (
            format!("{FIB_FN}Clip c:\n    for i in 0..fib(5):\n        Note/(60 + i)\n"),
            5,
            pitches(&[60, 61, 62, 63, 64]),
        ),
        (
            format!("{FIB_FN}let k = fib(4)\nClip c:\n    for i in 0..k:\n        Note\n"),
            3,
            Vec::new(),
        ),
        (
            format!("{FIB_FN}{arp}Clip c:\n    Arp/48 | steps=fib(4)\n"),
            3,
            pitches(&[48, 52, 56]),
        ),
        (
            format!("{FIB_FN}Clip c:\n    Note/([60, 62, 64][fib(3)])\n"),
            1,
            pitches(&[64]),
        ),
        (
            format!("{FIB_FN}let v = match fib(4):\n    3: 1\n    else: 2\nClip c:\n    Note/v\n"),
            1,
            pitches(&[1]),
        ),
        (
            format!("{FIB_FN}let v = match fib(3):\n    2: 1\n    else: 2\nClip c:\n    Note/v\n"),
            1,
            pitches(&[1]),
        ),
    ];
    let mut failures = Vec::new();
    for (text, count, expected) in structural {
        let (diagnostics, (found, holes)) = analyzed(&text, |engine| {
            (notes_and_pitches(engine), placeholders(engine))
        });
        if !errors_of(&diagnostics).is_empty() || found != (count, expected) || !holes.is_empty() {
            failures.push(format!(
                "{text:?}: {found:?} {holes:?} with {:?}",
                error_codes(&diagnostics)
            ));
        }
    }
    let cells: Vec<(String, &str, Scalar, Option<CVal>)> = vec![
        (
            format!("{FIB_FN}input n = fib(3)\nClip c:\n    Note/n\n"),
            "n",
            Scalar::I64,
            Some(CVal::Int(2)),
        ),
        (
            "fn h(n): if n < 1 then 0.5 else h(n - 1)\ninput m = h(2)\nClip c | gain=m\n".into(),
            "m",
            Scalar::F64,
            Some(CVal::F64(0.5)),
        ),
        (
            "fn d(n): if n < 1 then 1 else d(n - 1) / 2\ninput q = d(2)\nClip c | gain=q\n".into(),
            "q",
            Scalar::F64,
            Some(CVal::F64(0.25)),
        ),
        (
            format!(
                "{FIB_FN}input f = true\nlet x = if f then fib(3) else fib(4)\nClip c:\n    Note/x\n"
            ),
            "x",
            Scalar::I32,
            None,
        ),
    ];
    for (text, name, own, value) in cells {
        let (diagnostics, (typed, holes)) = analyzed(&text, |engine| {
            let own = engine
                .bindings
                .iter()
                .position(|binding| binding.name == name)
                .and_then(|position| u32::try_from(position).ok())
                .and_then(|position| engine.typing.own.get(&BindingId(position)).copied());
            (
                (
                    own,
                    cell_type_and_value(engine, name).map(|(_, value)| value),
                ),
                placeholders(engine),
            )
        });
        let value_holds = value.is_none() || typed.1 == value;
        if !errors_of(&diagnostics).is_empty()
            || typed.0 != Some(own)
            || !value_holds
            || !holes.is_empty()
        {
            failures.push(format!(
                "{text:?}: {typed:?} {holes:?} with {:?}",
                error_codes(&diagnostics)
            ));
        }
    }
    let rejected = [
        (
            "Clip c:\n    for i in 0..0.5:\n        Note\n".to_string(),
            Code::RangeBound,
        ),
        (
            "fn f(x): if x < 1 then x div 2 else f(x - 1)\nClip c:\n    Note/(f(0.5))\n".into(),
            Code::OperandType,
        ),
    ];
    for (text, code) in rejected {
        let (diagnostics, ()) = analyzed(&text, |_| ());
        if error_codes(&diagnostics) != [code] {
            failures.push(format!("{text:?}: {:?}", error_codes(&diagnostics)));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
