use symphonyscript_core::{Count, Limits, Scalar, Vocabulary};
use symphonyscript_fixtures::{
    Orders, Project, ProjectFile, bar_vocabulary, covered, deep_projects, dsp_project,
    envelope_vocabulary, music_project, pending_spec_examples, spec_examples, spec_projects,
    vocabulary,
};

use super::Engine;
use super::attr_value_source::AttrValueSource;
use super::binding::Binding;
use super::binding_kind::BindingKind;
use super::bound_val::BoundVal;
use super::c_array::CArray;
use super::c_val::CVal;
use super::chain::Chain;
use super::destination::Destination;
use super::eval_ty::EvalTy;
use super::ids::{BindingId, NodeId, ScopeId};
use super::kind::Kind;
use super::op::Op;
use super::query::Query;
use super::scope_kind::ScopeKind;
use super::val::Val;
use crate::compiler::Compiler;
use crate::diagnostics::{Code, Diagnostic, Severity};
use crate::semantics::Internal;
use crate::semantics::analyze;
use crate::semantics::collection::{Variants, collect};
use crate::semantics::ir::ExprId;
use crate::semantics::ir::{Hir, Loc, Top, build_file};
use crate::semantics::reporting::Report;
use crate::source::Source;
use crate::syntax::parse;

fn graph_size(calls: usize) -> (usize, usize) {
    let text = format!(
        "state i(1)\nfn twice(x): x + x\nClip c:\n    Note/({}i{})\n",
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
    build_file(&mut hir, &source.path, &parsed, &vocabulary).expect("the chain builds");
    let mut report = Report::default();
    let variants = Variants::of(&vocabulary).expect("the vocabulary's variants index");
    let collected = collect(&hir, &vocabulary, &variants, &mut report).expect("the chain collects");
    let mut engine = Engine::new(&hir, &vocabulary, &collected, &variants, Limits::default());
    engine.analyze().expect("the chain meets no internal error");
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
    let (diagnostics, found) = compiler.analyzed(&[source], inspect);
    (diagnostics, found.expect("the run meets no internal error"))
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
    for (text, value) in [
        ("Env e | attack=(3i32 * 2.5)\n", 0.0075),
        ("Env e | attack=(3i32 * 2)\n", 0.006),
    ] {
        let (diagnostics, found) = analyzed_with(&envelope, text, first_value);
        assert_eq!(diagnostics, Vec::<Diagnostic>::new(), "{text}");
        assert_eq!(
            found.map(|(found, _)| found),
            Some(CVal::F64(value)),
            "{text}"
        );
    }
}

#[test]
fn uses_of_a_flexible_value_compare_before_the_unit_reading() {
    let envelope = envelope_vocabulary();
    for (text, attack) in [
        ("let p = 3\nEnv e | attack=p pitch=p\n", 0.003),
        ("let p = 3\nEnv e | attack=(p + 1s) pitch=p\n", 1.0 + 0.003),
        ("let p = 3i32\nEnv e | attack=p pitch=p\n", 0.003),
    ] {
        let (diagnostics, found) = analyzed_with(&envelope, text, |engine| {
            (
                attr_value(engine, "attack", EvalTy::F64),
                attr_value(engine, "pitch", EvalTy::I32),
            )
        });
        assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new(), "{text}");
        assert_eq!(
            found,
            (Some(CVal::F64(attack)), Some(CVal::Int(3))),
            "{text}"
        );
    }
    let (diagnostics, attacks) = analyzed_with(
        &envelope,
        "let p = 3i32\nlet c = p > 1\nEnv e | attack=p\nEnv f | attack=(if c then 1s else 2s)\n",
        |engine| attr_values(engine, "attack", EvalTy::F64),
    );
    assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
    assert_eq!(attacks, vec![CVal::F64(0.003), CVal::F64(1.0)]);
    let (diagnostics, found) = analyzed(
        "let p = 16777217i32\nClip c | gain=p:\n    Note/p\n",
        |engine| {
            (
                attr_values(engine, "gain", EvalTy::F32),
                attr_values(engine, "pitch", EvalTy::I32),
            )
        },
    );
    assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
    assert_eq!(
        found,
        (vec![CVal::F32(16777216.0)], vec![CVal::Int(16777217)])
    );
    let (diagnostics, ()) = analyzed(
        "let q = 2 ** -1\nOsc o | freq=q\nClip c:\n    Note/q\n",
        |_| (),
    );
    assert_eq!(error_codes(&diagnostics), vec![Code::InconsistentUses]);
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
            format!("let k = ({family}) div 1bars\nBar b | bar_len=k\n"),
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
            "Clip c | gain=(16777217 / 2i32)\n".into(),
            "gain",
            EvalTy::F32,
            CVal::F32(8388608.0),
        ),
        (
            &example,
            "Clip c | gain=(16777217i32 / 2)\n".into(),
            "gain",
            EvalTy::F32,
            CVal::F32(8388608.0),
        ),
        (
            &example,
            "let x = 5\nClip c:\n    Note/(5i32 + x)\n".into(),
            "pitch",
            EvalTy::I32,
            CVal::Int(10),
        ),
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
            format!("{q}Bar b | bar_len=(q(0))\n"),
            "bar_len",
            EvalTy::I64,
            CVal::Int(4),
        ),
        (
            &bars,
            format!("{r}Bar b | bar_len=(r(2))\n"),
            "bar_len",
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
            format!("{r}Bar b | bar_len=(r(2)) beat_len=(r(2))\n"),
            "bar_len",
            EvalTy::I64,
            CVal::Int(12),
        ),
        (
            &bars,
            format!("{r}Bar b | bar_len=(r(2)) beat_len=(r(2))\n"),
            "beat_len",
            EvalTy::I64,
            CVal::Int(6),
        ),
        (
            &bars,
            format!("{r}Bar b | bar_len=(r(0) + 1)\n"),
            "bar_len",
            EvalTy::I64,
            CVal::Int(8),
        ),
        (
            &bars,
            "Bar b | bar_len=(1bars + 1)\n".into(),
            "bar_len",
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
        (format!("{q}Bar b | bar_len=(q(0))\n"), vec![]),
        ("let d = 2bars + 1\nBar b | bar_len=d\n".into(), vec![]),
        (
            "let d = 2bars + 1\nBar b | bar_len=d beat_len=d\n".into(),
            vec![Code::ConflictingUnits],
        ),
        (
            "Bar b | bar_len=42535295865117307932921825928971026432\n".into(),
            vec![Code::IntegerOverflow],
        ),
        (
            "Bar b | bar_len=(42535295865117307932921825928971026432 + 0)\n".into(),
            vec![Code::IntegerOverflow],
        ),
        (
            "Bar b | bar_len=42535295865117307932921825928971026431\n".into(),
            vec![Code::DoesNotFit],
        ),
        (
            "Bar b | bar_len=(-42535295865117307932921825928971026432 + 0)\n".into(),
            vec![Code::IntegerOverflow],
        ),
        (
            "Bar b | bar_len=(-42535295865117307932921825928971026431 + 0)\n".into(),
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
    let held = |scope: ScopeId| !engine.in_probe(scope).unwrap_or(false);
    let node_held = |node: NodeId| engine.node(node).map_or(true, |node| held(node.scope));
    let mut found = Vec::new();
    for ((expr, scope), val) in &memo.exprs {
        if matches!(val, Val::Error) && held(*scope) {
            found.push(format!(
                "value of {:?}",
                engine.hir.expr(*expr).map(|expr| expr.loc)
            ));
        }
    }
    for (node, sig) in &memo.sigs {
        if sig.is_error() && node_held(*node) {
            found.push(format!(
                "type of {:?}",
                engine.node(*node).map(|node| node.loc)
            ));
        }
    }
    for ((node, _), value) in &memo.evals {
        if value.is_error() && node_held(*node) {
            found.push(format!(
                "evaluation of {:?}",
                engine.node(*node).map(|node| node.loc)
            ));
        }
    }
    for (call, value) in &memo.calls {
        if value.is_error() {
            found.push(format!("result of {call:?}"));
        }
    }
    let locals = memo
        .lets
        .iter()
        .map(|((scope, _), bound)| (*scope, bound))
        .chain(
            memo.params
                .iter()
                .map(|((scope, _), bound)| (*scope, bound)),
        )
        .filter(|(scope, _)| held(*scope))
        .map(|(_, bound)| bound);
    let bounds = locals
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
    let pending = pending_spec_examples();
    let examples = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym" && !pending.contains(&example.name()))
        .map(|example| Project {
            name: example.name(),
            files: vec![ProjectFile {
                path: "example.sym".into(),
                text: example.text,
            }],
        });
    let projects = [music_project(), dsp_project()].into_iter().chain(examples);
    let clean = clean_runs_holding_no_placeholder(projects);
    assert!(
        clean
            >= 2 + spec_projects()
                .iter()
                .filter(|project| !pending.contains(&project.name))
                .count(),
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
        format!("{CHORD_FN}state f(true)\nstate a(1)\nstate i(0)\nClip c:\n    Note/((if f then chord(1) else [1, 2, 3])[0])\n    Note/([a, chord(3)[0]][1])\n    Note/(chord(3)[i])\n"),
        "fn g(k): if k < 1 then [[1, 2, 3], [4, 5, 6]] else g(k - 1)\nfn rev(a, k): if k < 1 then a else rev([a[1], a[0]], k - 1)\nfn grow(k): if k < 1 then [0] else [grow(k - 1)[0] + 1, 0]\nClip c:\n    Note/(g(2)[1][2] + rev([1, 2], 3)[0] + grow(3)[0])\n".into(),
    ];
    let held = "Session s\nstate f(true)\nlet x = if f then Sine else Saw\n";
    let enum_lets = [
        format!("{held}Clip c:\n    Note when x is Wave::Saw\n    Note when Wave::Saw is x\n"),
        format!("{held}state w(Wave::Saw)\nlet g = x is not w\nClip c:\n    Note when g\n"),
        format!(
            "{held}let y = if f then Saw else Sine\nOsc o | wave=x\nClip c:\n    Note when x is y\n    Note when x is Saw\n"
        ),
    ];
    for text in recursive_arrays.iter().chain(&enum_lets) {
        let (diagnostics, found) = analyzed(text, placeholders);
        assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new(), "{text}");
        assert_eq!(found, Vec::<String>::new(), "{text}");
    }
}

fn clean_runs_holding_no_placeholder(projects: impl Iterator<Item = Project> + Send) -> usize {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    std::thread::scope(|threads| {
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
                    let found = found.expect("the run meets no internal error");
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
    })
}

#[test]
#[ignore = "stress"]
fn stress_a_run_over_a_deep_project_draws_no_error_and_holds_no_placeholder() {
    assert_eq!(
        clean_runs_holding_no_placeholder(deep_projects().into_iter()),
        4
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
        match engine.node(node).expect("the node exists").op {
            Op::Use(inner) => node = inner,
            Op::Read(binding) => {
                let record = engine.binding(binding).expect("the binding exists");
                return match record.kind {
                    BindingKind::Input { input, element } => {
                        let name = engine
                            .hir
                            .input(input)
                            .expect("the input exists")
                            .name
                            .as_ref();
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
                .expect("the placement exists")
                .gate
                .map(|gate| reads(engine, gate, instance.scope));
            (pitch, gate)
        })
        .collect()
}

#[test]
fn an_index_known_at_compile_time_reads_the_elements_own_binding() {
    let text = "state a(1)\nstate b(2)\nstate j(0)\nlet c = [a, b, a + b]\nlet k = 1\nClip m:\n    Note/(c[1])\n    Note/(c[k])\n    for i in 1..2:\n        Note/(c[i])\n    Note/(c[2])\n    Note/(c[j])\n";
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
    let text = "state steps([true, false, false, true, false, false, true, false])\nstate pitches([36, 36, 43, 36, 41, 36, 43, 48])\n\nClip seq:\n    for i in 0..8:\n        Note/(pitches[i]) when steps[i]\n";
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
                .expect("every condition link exists")
                .into_iter()
                .map(|condition| {
                    let loc = engine
                        .hir
                        .expr(condition.expr)
                        .expect("the guard exists")
                        .loc;
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
    let text = "state fill_on(false)\nstate section(0)\nClip verse:\n    Note/60\n    when fill_on:\n        Note/62 Note/64\n    else:\n        Note/62\n    when:\n        section is 0: Note/67\n        section is 1: Note/69\n        else: Note/71\n";
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
}

#[test]
#[ignore = "stress"]
fn stress_an_arm_of_the_deep_bodies_project_is_active_when_every_guard_above_it_holds() {
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
#[ignore = "stress"]
fn stress_guard_arms_cost_linear_time_and_memory() {
    let size = |arms: usize| {
        let mut text = String::from("state x(0)\nClip c:\n    when:\n");
        for arm in 0..arms {
            text.push_str(&format!("        x is {arm}: Note/{arm}\n"));
        }
        let (diagnostics, size) = analyzed(&text, |engine| {
            engine.nodes.len()
                + engine.condition_links.len()
                + engine.gates.len()
                + engine.typing.gate_positions
                + engine.typing.checks
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
        let mut text = String::from("state i(1)\nfn f0(x): x\n");
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
        format!("{g}Bar b | bar_len=(g()) beat_len=(g())\n"),
        format!("{g}let x = g()\nlet y = g()\nBar b | bar_len=x beat_len=y\n"),
    ] {
        let (diagnostics, found) = analyzed_with(&bars, &text, |engine| {
            (
                attr_value(engine, "bar_len", EvalTy::I64),
                attr_value(engine, "beat_len", EvalTy::I64),
            )
        });
        assert_eq!(error_codes(&diagnostics), Vec::<Code>::new(), "{text}");
        assert_eq!(found, (Some(CVal::Int(8)), Some(CVal::Int(5))), "{text}");
    }
    let half = "fn g(): 2 ** -1\n";
    for text in [
        format!("{half}Clip c:\n    Note/(g())\nOsc o | freq=(g())\n"),
        format!("{half}let x = g()\nlet z = g()\nClip c:\n    Note/x\nOsc o | freq=z\n"),
    ] {
        let (diagnostics, found) = analyzed(&text, |engine| {
            (
                attr_values(engine, "pitch", EvalTy::I32),
                attr_values(engine, "freq", EvalTy::F64),
            )
        });
        assert_eq!(error_codes(&diagnostics), Vec::<Code>::new(), "{text}");
        assert_eq!(found, (vec![CVal::Int(0)], vec![CVal::F64(0.5)]), "{text}");
    }
    let held = format!("{half}let q = g()\nClip c:\n    Note/q\nOsc o | freq=q\n");
    let (diagnostics, ()) = analyzed(&held, |_| ());
    assert_eq!(error_codes(&diagnostics), vec![Code::InconsistentUses]);
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
        .filter(|((node, _), _)| {
            engine
                .node(*node)
                .is_ok_and(|record| matches!(record.op, Op::Recursive { .. }))
        })
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
                    .related
                    .iter()
                    .map(|related| &related.span)
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
            format!("{CHORD_FN}state a(1)\nClip c:\n    Note/(a + chord(1))\n"),
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
            format!("{CHORD_FN}state f(true)\nClip c:\n    Note/((if f then chord(1) else [1, 2, 3])[0])\n"),
            vec![],
            vec![],
        ),
        (
            format!("{CHORD_FN}state f(true)\nClip c:\n    Note/((if f then chord(1) else [1, 2])[0])\n"),
            vec![],
            vec![],
        ),
        (
            format!("{CHORD_FN}state a(1)\nClip c:\n    Note/([a, chord(3)[0]][1])\n"),
            vec![],
            vec![62],
        ),
        (
            format!("{CHORD_FN}state i(0)\nClip c:\n    Note/(chord(3)[i])\n"),
            vec![],
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
                "{CHORD_FN}state f(true)\nChord a | notes=(if f then chord(1) else chord(2))\n"
            ),
            "notes",
            vec![],
            None,
        ),
        (
            format!("{CHORD_FN}state f(true)\nChord a | notes=(if f then chord(1) else [1, 2])\n"),
            "notes",
            vec![Code::VariableArray],
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
                "{CHORD_FN}state f(true)\nChord a | notes=(if f then chord(1) else chord(2)[0])\n"
            ),
            "notes",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            format!("{grid}state i(0)\nChord a | notes=(g(1)[i])\n"),
            "notes",
            vec![],
            None,
        ),
        (
            format!("{grid}state i(0)\nChord a | notes=(g(1)[i][0])\n"),
            "notes",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            format!("{grid}state f(true)\nChord a | grid=(if f then g(1) else g(2))\n"),
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
    let annotated = format!("{CHORD_FN}state v(chord(1)): i32[3]\nClip c:\n    Note/(v[2])\n");
    let (diagnostics, holes) = analyzed(&annotated, placeholders);
    if !errors_of(&diagnostics).is_empty() || !holes.is_empty() {
        failures.push(format!(
            "{annotated:?}: {holes:?} with {:?}",
            error_codes(&diagnostics)
        ));
    }
    let wrong = format!("{CHORD_FN}state v(chord(1)): i32[2]\nClip c:\n    Note/(v[1])\n");
    let (diagnostics, ()) = analyzed(&wrong, |_| ());
    if error_codes(&diagnostics) != [Code::TypeMismatch] {
        failures.push(format!("{wrong:?}: {:?}", error_codes(&diagnostics)));
    }
    let input = format!("{CHORD_FN}state v(chord(1))\nClip c:\n    Note/(v[2])\n");
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

fn a_placement_for_over_a_doubled_array_stops_at_the_expansion_limit(depth: usize) {
    let text = format!("{DOUBLING}Clip c:\n    for x in f({depth}):\n        Note/x\n");
    let (diagnostics, built) = analyzed(&text, |engine| engine.memo.iterated_elements);
    assert_eq!(error_codes(&diagnostics), [Code::ExpansionLimit], "{text}");
    assert!(built <= 1_000_001, "{text}: {built}");
}

const DOUBLING: &str = "fn f(n): if n is 0 then [1] else f(n - 1) + f(n - 1)\n";

#[test]
#[ignore = "stress"]
fn stress_a_placement_for_over_two_to_the_thirty_elements_builds_no_more_than_the_expansion_limit()
{
    a_placement_for_over_a_doubled_array_stops_at_the_expansion_limit(30);
}

#[test]
#[ignore = "stress"]
fn stress_a_placement_for_over_two_to_the_twenty_two_elements_builds_no_more_than_the_expansion_limit()
 {
    a_placement_for_over_a_doubled_array_stops_at_the_expansion_limit(22);
}

#[test]
#[ignore = "stress"]
fn stress_a_placement_for_over_a_range_past_the_expansion_limit_draws_expansion_limit() {
    let range = "Clip c:\n    for i in 0..8388608:\n        Note\n";
    let (diagnostics, ()) = analyzed(range, |_| ());
    assert_eq!(error_codes(&diagnostics), [Code::ExpansionLimit], "{range}");
}

#[test]
fn a_placement_for_over_a_compile_time_array_builds_its_elements_as_it_iterates() {
    let doubling = DOUBLING;
    let rows: [(String, usize, Vec<CVal>); 2] = [
        (
            format!("{doubling}Clip c:\n    for x in f(3):\n        Note/(60 + x)\n"),
            8,
            vec![CVal::Int(61); 8],
        ),
        (
            "Clip c:\n    for x in [60, 64, 67]:\n        Note/x\n".to_string(),
            3,
            vec![CVal::Int(60), CVal::Int(64), CVal::Int(67)],
        ),
    ];
    for (text, notes, pitches) in rows {
        let (diagnostics, found) = analyzed(&text, notes_and_pitches);
        assert!(errors_of(&diagnostics).is_empty(), "{text}{diagnostics:?}");
        assert_eq!(found, (notes, pitches), "{text}");
    }
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
        Scalar::U8 => EvalTy::U8,
        Scalar::I32 => EvalTy::I32,
        Scalar::I64 => EvalTy::I64,
        Scalar::F32 => EvalTy::F32,
        Scalar::F64 => EvalTy::F64,
        Scalar::Bool | Scalar::Enum(_) | Scalar::Str => return None,
    };
    let init = engine.binding(binding).ok()?.init;
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
            format!("{FIB_FN}state n(fib(3))\nClip c:\n    Note/n\n"),
            "n",
            Scalar::I64,
            Some(CVal::Int(2)),
        ),
        (
            "fn h(n): if n < 1 then 0.5 else h(n - 1)\nstate m(h(2))\nClip c | gain=m\n".into(),
            "m",
            Scalar::F64,
            Some(CVal::F64(0.5)),
        ),
        (
            "fn d(n): if n < 1 then 1 else d(n - 1) / 2\nstate q(d(2))\nClip c | gain=q\n".into(),
            "q",
            Scalar::F64,
            Some(CVal::F64(0.25)),
        ),
        (
            format!(
                "{FIB_FN}state f(true)\nlet x = if f then fib(3) else fib(4)\nClip c:\n    Note/x\n"
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

const GRID_FN: &str = "fn grid(k): if k < 1 then [[1, 2], [3, 4]] else grid(k - 1)\n";

type DefaultShapeRow<'a> = (String, &'a str, usize, Vec<(EvalTy, CVal)>);

fn element_types(engine: &Engine<'_>, name: &str) -> Vec<Option<Scalar>> {
    engine
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, binding)| binding.name == name)
        .map(|(position, _)| {
            u32::try_from(position)
                .ok()
                .and_then(|position| engine.typing.own.get(&BindingId(position)).copied())
        })
        .collect()
}

#[test]
fn a_reactive_value_shaped_by_a_recursive_result_alone_takes_its_default_shape() {
    let row = |values: &[i128]| {
        CVal::Array(CArray::new(
            values.iter().map(|value| CVal::Int(*value)).collect(),
        ))
    };
    let accepted: Vec<DefaultShapeRow<'_>> = vec![
        (
            format!(
                "{CHORD_FN}state f(true)\nlet x = if f then chord(1) else chord(2)\nClip c:\n    Note/(x[0])\n"
            ),
            "x",
            3,
            vec![
                (EvalTy::I32, row(&[62, 65, 69])),
                (EvalTy::I32, row(&[60, 64, 67])),
            ],
        ),
        (
            format!(
                "{CHORD_FN}fn first(a): a[0]\nstate f(true)\nClip c:\n    Note/(first(if f then chord(1) else chord(2)))\n"
            ),
            "a",
            3,
            vec![
                (EvalTy::I32, row(&[62, 65, 69])),
                (EvalTy::I32, row(&[60, 64, 67])),
            ],
        ),
        (
            format!("{GRID_FN}state i(0)\nlet row = grid(2)[i]\nClip c:\n    Note/(row[1])\n"),
            "row",
            2,
            vec![(
                EvalTy::I32,
                CVal::Array(CArray::new(vec![row(&[1, 2]), row(&[3, 4])])),
            )],
        ),
        (
            format!(
                "{R_FN}state f(true)\nlet w = if f then r(0.1, 3) else r(0.1, 3)\nClip c | gain=(w[0])\n"
            ),
            "w",
            1,
            vec![
                (EvalTy::F32, CVal::Array(CArray::new(vec![CVal::F32(0.1)]))),
                (EvalTy::F32, CVal::Array(CArray::new(vec![CVal::F32(0.1)]))),
            ],
        ),
    ];
    let mut failures = Vec::new();
    for (text, name, elements, results) in accepted {
        let (diagnostics, (types, found, holes)) = analyzed(&text, |engine| {
            (
                element_types(engine, name),
                recursive_results(engine),
                placeholders(engine),
            )
        });
        let used = results.first().map(|(ty, _)| match ty {
            EvalTy::F32 => Scalar::F32,
            _ => Scalar::I32,
        });
        let computed = results.iter().all(|result| found.contains(result));
        if !errors_of(&diagnostics).is_empty()
            || types.len() != elements
            || !types.contains(&used)
            || !computed
            || !holes.is_empty()
        {
            failures.push(format!(
                "{text:?}: {types:?} {found:?} {holes:?} with {:?}",
                error_codes(&diagnostics)
            ));
        }
    }
    let annotated = format!(
        "{CHORD_FN}state f(true)\nlet x: i32[3] = if f then chord(1) else chord(2)\nClip c:\n    Note/(x[2])\n"
    );
    let (diagnostics, (types, holes)) = analyzed(&annotated, |engine| {
        (element_types(engine, "x"), placeholders(engine))
    });
    if !errors_of(&diagnostics).is_empty() || types != [Some(Scalar::I32); 3] || !holes.is_empty() {
        failures.push(format!(
            "{annotated:?}: {types:?} {holes:?} with {:?}",
            error_codes(&diagnostics)
        ));
    }
    let mismatched = format!(
        "{CHORD_FN}state f(true)\nlet x: i32[2] = if f then chord(1) else chord(2)\nClip c:\n    Note/(x[1])\n"
    );
    let (diagnostics, ()) = analyzed(&mismatched, |_| ());
    if error_codes(&diagnostics) != [Code::TypeMismatch] {
        failures.push(format!("{mismatched:?}: {:?}", error_codes(&diagnostics)));
    }
    let other = "fn other(k): if k < 1 then [1, 2] else other(k - 1)\n";
    let variable = format!(
        "{CHORD_FN}{other}state f(true)\nlet x = if f then chord(1) else other(1)\nClip c:\n    Note/(x[1])\n"
    );
    let (diagnostics, ()) = analyzed(&variable, |_| ());
    if !errors_of(&diagnostics).is_empty() {
        failures.push(format!("{variable:?}: {:?}", error_codes(&diagnostics)));
    }
    let rejected = format!(
        "{R_FN}state f(true)\nlet w = if f then r(0.1, 3) else r(0.1, 3)\nOsc o | freq=(w[0])\n"
    );
    let (diagnostics, ()) = analyzed(&rejected, |_| ());
    let used = rejected.find("w[0]").unwrap_or(0);
    let declared = rejected.find("let w").unwrap_or(0);
    let found = located(&rejected, &diagnostics);
    if found != [(Code::InconsistentShapes, used, vec![declared])] {
        failures.push(format!("{rejected:?}: found {found:?}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn doubling(name: &str, levels: usize, left: &str, right: &str) -> String {
    let mut text = format!("state i(1)\nfn {name}0(x): x\n");
    for level in 1..=levels {
        let below = level - 1;
        text.push_str(&format!(
            "fn {name}{level}(x): {name}{below}({left}) + {name}{below}({right})\n"
        ));
    }
    text.push_str(&format!("Clip c:\n    Note/({name}{levels}(i))\n"));
    text
}

fn inlined_bodies(engine: &Engine<'_>) -> usize {
    engine
        .scopes
        .iter()
        .filter(|scope| {
            matches!(&scope.kind, super::scope_kind::ScopeKind::Locals(locals)
                if matches!(locals.owner, super::locals_owner::LocalsOwner::Fn(_))
                    && !locals.recursive)
        })
        .count()
}

#[test]
fn a_compilation_inlines_at_most_a_million_calls() {
    let work = |text: &str| {
        let (diagnostics, sizes) = analyzed(text, |engine| {
            (
                engine.nodes.len() + engine.scopes.len() + engine.bindings.len(),
                inlined_bodies(engine),
                engine.inlined,
            )
        });
        (diagnostics, sizes)
    };
    let (diagnostics, (accepted_work, bodies, counted)) = work(&doubling("f", 18, "x", "x"));
    assert_eq!(error_codes(&diagnostics), Vec::<Code>::new());
    assert_eq!((bodies, counted), (19, (1 << 19) - 1));
    for text in [
        doubling("f", 19, "x", "x"),
        doubling("g", 19, "x * 2", "x * 3"),
        doubling("g", 40, "x * 2", "x * 3"),
    ] {
        let (diagnostics, (sizes, bodies, _)) = work(&text);
        assert_eq!(error_codes(&diagnostics), vec![Code::InlineLimit], "{text}");
        assert_eq!(bodies, 0, "{text}");
        assert!(
            sizes <= accepted_work,
            "{sizes} against {accepted_work}: {text}"
        );
    }
    let mut sites = String::from("state i(1)\nfn g0(x): x\n");
    for level in 1..=10 {
        let below = level - 1;
        sites.push_str(&format!(
            "fn g{level}(x): g{below}(x * 2) + g{below}(x * 3)\n"
        ));
    }
    sites.push_str("Clip c:\n    for j in 0..1000:\n        Note/(g10(i + j))\n");
    let (diagnostics, (_, bodies, _)) = work(&sites);
    let call = sites.find("g10(i + j)").unwrap_or(usize::MAX);
    assert_eq!(
        located(&sites, &diagnostics),
        vec![(Code::InlineLimit, call, vec![])]
    );
    assert_eq!(bodies, 0);
}

#[test]
fn the_count_before_inlining_is_the_count_of_every_construct() {
    let mut text = String::from("state i(1)\nfn f0(x): x\n");
    for level in 1..=18 {
        let below = level - 1;
        text.push_str(&format!("fn f{level}(x): f{below}(x) + f{below}(x)\n"));
    }
    text.push_str(concat!(
        "let a = f18(i)\n",
        "pattern P(default root, extra = f15(i)):\n",
        "    let b = f16(i)\n",
        "    Note/(root + b + extra)\n",
        "Clip c:\n",
        "    P/(f17(i))\n",
        "    Note/a when f13(i) > 0\n",
        "    Plays | gain=(f8(i)): c\n",
        "    for j in 0..1:\n",
        "        Note/(f4(f4(f0(i + j))))\n",
        "Clip d:\n",
        "    when f2(i) > 0:\n",
        "        Note\n",
    ));
    let measure = |text: &str| analyzed(text, |engine| (inlined_bodies(engine), engine.inlined));
    let (diagnostics, (_, counted)) = measure(&text);
    assert_eq!(error_codes(&diagnostics), Vec::<Code>::new());
    assert_eq!(counted, 1_000_000);
    let past = format!("{text}        Note/(f0(i))\n");
    let (diagnostics, (bodies, _)) = measure(&past);
    let crossing = past.find("f16(i)").unwrap_or(usize::MAX);
    let used = past.find("P/(f17(i))").unwrap_or(usize::MAX);
    assert_eq!(
        located(&past, &diagnostics),
        vec![(Code::InlineLimit, crossing, vec![used])]
    );
    assert_eq!(bodies, 0);
}

fn million_inlinings(f0_notes: usize, declarations: &str, values: &[&str]) -> String {
    let mut text = String::from("state i(1)\nfn f0(x): x\n");
    for level in 1..=18 {
        let below = level - 1;
        text.push_str(&format!("fn f{level}(x): f{below}(x) + f{below}(x)\n"));
    }
    text.push_str(declarations);
    text.push_str("Clip c:\n");
    for level in [18, 17, 16, 15, 13, 8, 5] {
        text.push_str(&format!("    Note/(f{level}(i))\n"));
    }
    for _ in 0..f0_notes {
        text.push_str("    Note/(f0(i))\n");
    }
    for value in values {
        text.push_str(&format!("    Note/({value})\n"));
    }
    text
}

#[test]
fn a_crossing_of_the_inline_limit_is_reported_at_the_call_that_crosses_it() {
    let measure = |text: &str| analyzed(text, |engine| (inlined_bodies(engine), engine.inlined));
    let (diagnostics, (_, counted)) = measure(&million_inlinings(7, "", &[]));
    assert_eq!(error_codes(&diagnostics), Vec::<Code>::new());
    assert_eq!(counted, 1_000_000);
    let wide = "fn r(n): if n < 1 then f0(n) + 3000000000 else r(n - 1)\n";
    let past = million_inlinings(7, wide, &["r(0)"]);
    let (diagnostics, (bodies, _)) = measure(&past);
    let call = past.find("r(0)").unwrap_or(usize::MAX);
    assert_eq!(
        located(&past, &diagnostics),
        vec![(Code::InlineLimit, call, vec![])]
    );
    assert_eq!(bodies, 0);
    let under = million_inlinings(4, wide, &["r(0)"]);
    let (diagnostics, (_, counted)) = measure(&under);
    let call = under.find("r(0)").unwrap_or(usize::MAX);
    assert_eq!(
        located(&under, &diagnostics),
        vec![(Code::DoesNotFit, call, vec![])]
    );
    assert_eq!(counted, 999_998);
    let defaulted = "fn r(n, k = f0(1)): if n < 1 then k else r(n - 1)\n";
    let late = million_inlinings(7, defaulted, &["r(1, 2)"]);
    let (diagnostics, (bodies, _)) = measure(&late);
    let call = late.find("r(1, 2)").unwrap_or(usize::MAX);
    assert_eq!(
        located(&late, &diagnostics),
        vec![(Code::InlineLimit, call, vec![])]
    );
    assert_eq!(bodies, 0);
}

#[test]
fn a_call_in_a_recursive_body_counts_once() {
    let recursive = "fn r(n): if n < 1 then f0(n) else r(n - 1)\n";
    let accepted = million_inlinings(6, recursive, &["r(0)", "r(3)"]);
    let measure = |text: &str| analyzed(text, |engine| (inlined_bodies(engine), engine.inlined));
    let (diagnostics, (_, counted)) = measure(&accepted);
    assert_eq!(error_codes(&diagnostics), Vec::<Code>::new());
    assert_eq!(counted, 1_000_000);
    let past = million_inlinings(7, recursive, &["r(0)", "r(3)"]);
    let (diagnostics, (bodies, _)) = measure(&past);
    let call = past.find("r(0)").unwrap_or(usize::MAX);
    assert_eq!(
        located(&past, &diagnostics),
        vec![(Code::InlineLimit, call, vec![])]
    );
    assert_eq!(bodies, 0);
}

fn chains(f_levels: usize, g_levels: usize, declarations: &str, notes: &[&str]) -> String {
    let mut text = String::from("state i(1)\nfn f0(x): x\nfn g0(x): x\n");
    for level in 1..=f_levels {
        let below = level - 1;
        text.push_str(&format!("fn f{level}(x): f{below}(x) + f{below}(x)\n"));
    }
    for level in 1..=g_levels {
        let below = level - 1;
        text.push_str(&format!(
            "fn g{level}(x): g{below}(x * 2) + g{below}(x * 3)\n"
        ));
    }
    text.push_str(declarations);
    text.push_str("Clip c:\n");
    for note in notes {
        text.push_str(&format!("    {note}\n"));
    }
    text
}

#[test]
fn the_count_before_inlining_follows_the_defaults_each_call_leaves_open() {
    let measure = |text: &str| analyzed(text, |engine| (inlined_bodies(engine), engine.inlined));
    let failures = std::cell::RefCell::new(Vec::new());
    let rejected_at = |text: &str, call: &str| {
        let (diagnostics, (bodies, _)) = measure(text);
        let at = text.rfind(call).unwrap_or(usize::MAX);
        let found = (located(text, &diagnostics), bodies);
        if found != (vec![(Code::InlineLimit, at, vec![])], 0) {
            failures
                .borrow_mut()
                .push(format!("{call}: found {found:?}"));
        }
    };
    let accepted = |text: &str, label: &str, expected: u64| {
        let (diagnostics, (_, counted)) = measure(text);
        let found = (error_codes(&diagnostics), counted);
        if found != (Vec::new(), expected) {
            failures
                .borrow_mut()
                .push(format!("{label}: found {found:?}"));
        }
    };
    let defaulted = "fn r(n, d = f17(n)): if n < 1 then d else r(n - 1)\n";
    for call in ["r(3)", "r(3, 1)"] {
        let note = format!("Note/({call})");
        let text = chains(
            17,
            10,
            defaulted,
            &["for j in 0..400:", "    Note/(g10(i + j))", &note],
        );
        rejected_at(&text, call);
    }
    let base = [
        "Note/(f18(i))",
        "Note/(f16(i))",
        "Note/(f15(i))",
        "Note/(f13(i))",
        "Note/(f8(i))",
        "Note/(f5(i))",
        "Note/(f2(i))",
    ];
    for calls in [&["r(3)"][..], &["r(3, 1)"], &["r(3, 1)", "r(3)"]] {
        let mut notes: Vec<String> = base.iter().map(|note| (*note).to_string()).collect();
        notes.extend(calls.iter().map(|call| format!("Note/({call})")));
        let held: Vec<&str> = notes.iter().map(String::as_str).collect();
        accepted(
            &chains(18, 0, defaulted, &held),
            &format!("{calls:?}"),
            1_000_000,
        );
        let mut past = held.clone();
        past.push("Note/(f0(i))");
        rejected_at(&chains(18, 0, defaulted, &past), "f0(i)");
    }
    let reaching = concat!(
        "fn r(n): if n < 1 then 0 else r(n - 1) + f19(n)\n",
        "fn g(x, d = r(1)): f18(x) + d\n",
        "fn k(x): g(x)\n",
        "fn s(n, d = r(1)): if n < 1 then d else s(n - 1)\n",
    );
    for call in ["g(1)", "k(1)", "s(1)"] {
        let note = format!("Note/({call})");
        rejected_at(&chains(19, 0, reaching, &[&note]), call);
    }
    accepted(
        &chains(19, 0, reaching, &["Note/(g(1, 2))"]),
        "g(1, 2)",
        1 << 19,
    );
    let failures = failures.into_inner();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
#[ignore = "stress"]
fn stress_expansion_stops_after_a_million_iterations_and_uses_whatever_the_loop_bounds() {
    let mut chain = String::new();
    for level in 1..25 {
        let next = level + 1;
        chain.push_str(&format!("pattern P{level}():\n    P{next}\n    P{next}\n"));
    }
    chain.push_str("pattern P25():\n    for j in 0..0:\n        Note\nClip c:\n    P1\n");
    let unbounded =
        "Clip c:\n    for i in 0..1000000000000:\n        for j in 0..0:\n            Note\n";
    for text in [unbounded, chain.as_str()] {
        let (diagnostics, (steps, work)) = analyzed(text, |engine| {
            (
                engine.steps,
                engine.nodes.len() + engine.scopes.len() + engine.uses.len(),
            )
        });
        assert_eq!(
            error_codes(&diagnostics),
            vec![Code::ExpansionLimit],
            "{text}"
        );
        assert_eq!(steps, 1_000_001, "{text}");
        let work = u64::try_from(work).expect("the work count fits u64");
        assert!(work <= 3 * steps + 16, "{work} for {steps} steps: {text}");
    }
}

fn doubled_calls(levels: usize, body: &str) -> String {
    let mut text = format!("state i(1)\nfn g0(x): {body}\n");
    for level in 1..=levels {
        let below = level - 1;
        text.push_str(&format!("fn g{level}(x): g{below}(x) + g{below}(x)\n"));
    }
    text.push_str(&format!("Clip c:\n    Note/(g{levels}(i))\n"));
    text
}

#[test]
fn a_fault_in_shared_bodies_costs_what_the_clean_program_costs() {
    let measure = |text: &str| {
        analyzed(text, |engine| {
            (
                engine.nodes.len(),
                engine.scopes.len(),
                engine.bindings.len(),
                engine.memo.query_runs,
            )
        })
    };
    let faulty = doubled_calls(18, "x div 0.5");
    let clean = doubled_calls(18, "x div 2");
    let (diagnostics, faulty_sizes) = measure(&faulty);
    let (clean_diagnostics, clean_sizes) = measure(&clean);
    assert_eq!(error_codes(&clean_diagnostics), Vec::<Code>::new());
    let errors = errors_of(&diagnostics);
    assert_eq!(error_codes(&diagnostics), vec![Code::OperandType]);
    let chain: Vec<String> = errors
        .first()
        .map(|diagnostic| {
            diagnostic
                .related
                .iter()
                .map(|related| &related.span)
                .filter_map(|span| faulty.get(span.range.start as usize..span.range.end as usize))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mut expected: Vec<String> = (0..18).map(|level| format!("g{level}(x)")).collect();
    expected.push("g18(i)".into());
    assert_eq!(chain, expected);
    assert!(faulty_sizes.3 <= clean_sizes.3 * 2, "queries");
    assert!(faulty_sizes.0 <= clean_sizes.0 * 2, "nodes");
    assert!(faulty_sizes.1 <= clean_sizes.1 * 2, "scopes");
    assert!(faulty_sizes.2 <= clean_sizes.2 * 2, "bindings");
}

fn built(vocabulary: &Vocabulary, text: &str) -> Hir {
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let mut hir = Hir::default();
    hir.seal_prelude();
    let parsed = parse(&source, vocabulary);
    build_file(&mut hir, &source.path, &parsed, vocabulary).expect("the program builds");
    hir
}

#[test]
fn a_lookup_that_misses_is_an_internal_error() {
    let vocabulary = vocabulary();
    let variants = Variants::of(&vocabulary).expect("the vocabulary's variants index");
    let other = built(
        &vocabulary,
        "state a(1)\nstate b(2)\nstate c(3)\nSession s\n",
    );
    let Some(&Top::Input(foreign_input)) = other.top.get(2) else {
        panic!("the other program declares three inputs");
    };
    let mut hir = built(&vocabulary, "Session s\n");
    hir.top.push(Top::Input(foreign_input));
    let mut report = Report::default();
    let (found, _) = analyze::program(
        &hir,
        &vocabulary,
        &variants,
        Limits::default(),
        &mut report,
        |_| (),
    );
    let codes: Vec<Code> = report
        .finish(&hir.files)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(found, None);
    assert_eq!(codes, vec![Code::InternalError]);

    let (_, larger) = analyzed(
        "state i(1)\nClip c:\n    Note/(i + i + i)\n    Note/(i * 2)\n",
        |engine| engine.nodes.len(),
    );
    let hir = built(&vocabulary, "Session s\n");
    let mut report = Report::default();
    let collected = collect(&hir, &vocabulary, &variants, &mut report).expect("collects");
    let mut engine = Engine::new(&hir, &vocabulary, &collected, &variants, Limits::default());
    engine
        .analyze()
        .expect("the program meets no internal error");
    assert!(engine.nodes.len() < larger);
    let foreign = NodeId(u32::try_from(larger - 1).expect("the arena is small"));
    let scope = engine.source_global;
    let user = engine
        .push_node(Op::Use(foreign), Loc::default(), scope)
        .expect("the user node is pushed");
    assert_eq!(
        engine.demand_sig(user).map(|sig| sig.kind),
        Err(Internal::missed("node"))
    );

    let fresh = engine
        .push_node(Op::Bool(true), Loc::default(), scope)
        .expect("the fresh node is pushed");
    engine.run_query = |_, _, _| Ok(false);
    assert_eq!(
        engine.demand_sig(fresh).map(|sig| sig.kind),
        Err(Internal::missed("query result"))
    );
    engine.run_query = |_, _, _| Ok(true);
    assert_eq!(
        engine.demand_sig(fresh).map(|sig| sig.kind),
        Err(Internal::missed("demanded signature"))
    );
    assert_eq!(
        engine.demand_cval(fresh, EvalTy::Exact),
        Err(Internal::missed("demanded compile-time value"))
    );
    assert_eq!(
        engine.demand_val(Query::Expr(ExprId(0), scope)),
        Err(Internal::missed("demanded value"))
    );

    let empty = Vocabulary::builder("empty")
        .build()
        .expect("the empty vocabulary builds");
    let rows = [
        ("Clip c\n", Stage::Expand, "node kind"),
        ("let g = c.gain\nClip c\n", Stage::Infer, "node kind"),
        ("Clip c | gain=0.5\n", Stage::Infer, "node kind"),
        ("Clip c\n", Stage::Check, "node kind"),
        (
            "Clip a:\n    Plays: b\nClip b\n",
            Stage::CheckWithoutDestinations,
            "destination",
        ),
    ];
    for (text, stage, missed) in rows {
        assert_eq!(
            staged(&vocabulary, &empty, text, stage),
            Err(Internal::missed(missed)),
            "{text}"
        );
        let valid = match stage {
            Stage::CheckWithoutDestinations => Stage::Check,
            Stage::Expand | Stage::Infer | Stage::Check => stage,
        };
        assert_eq!(
            staged(&vocabulary, &vocabulary, text, valid),
            Ok(()),
            "{text}"
        );
    }

    with_engine("Session s\nlet a = [1, 2]\n", |engine| {
        assert_eq!(engine.analyze(), Ok(()));
        let (&(expr, _), &group) = engine
            .memo
            .literal_groups
            .iter()
            .next()
            .expect("the program holds a literal");
        let unknown = ScopeId(u32::MAX);
        engine.memo.literal_groups.insert((expr, unknown), group);
        assert_eq!(
            engine.held(expr, Some(Val::Error), unknown, &mut Vec::new()),
            Err(Internal::missed("scope"))
        );
    });
}

pub(super) fn with_engine<R>(text: &str, run: impl FnOnce(&mut Engine<'_>) -> R) -> R {
    with_engine_within(text, Limits::default(), run)
}

fn with_engine_within<R>(text: &str, limits: Limits, run: impl FnOnce(&mut Engine<'_>) -> R) -> R {
    let vocabulary = vocabulary();
    let variants = Variants::of(&vocabulary).expect("the vocabulary's variants index");
    let hir = built(&vocabulary, text);
    let mut report = Report::default();
    let collected =
        collect(&hir, &vocabulary, &variants, &mut report).expect("the program collects");
    let mut engine = Engine::new(&hir, &vocabulary, &collected, &variants, limits);
    run(&mut engine)
}

#[test]
fn reaching_an_id_space_draws_a_limit_diagnostic_and_no_internal_error() {
    let unbounded = Limits {
        expanded_items: u64::MAX,
        inlined_calls: u64::MAX,
        ..Limits::default()
    };
    let found = |engine: &Engine<'_>| -> Vec<(Code, String)> {
        engine
            .report
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.code, diagnostic.summary.clone()))
            .collect()
    };
    let program =
        "state i(1)\nfn f(x): x + x\nClip c:\n    for j in 0..100:\n        Note/(f(i + j))\n";
    with_engine_within(program, unbounded, |engine| {
        assert_eq!(engine.analyze(), Ok(()));
        assert_eq!(error_codes(&engine.report.diagnostics), Vec::<Code>::new());
    });
    with_engine_within(program, unbounded, |engine| {
        engine.id_space = 300;
        assert_eq!(engine.analyze(), Ok(()));
        let diagnostics = found(engine);
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(
            diagnostics
                .iter()
                .all(|(code, message)| *code == Code::ExpansionLimit
                    && message.contains("more than 300 ")),
            "{diagnostics:?}"
        );
    });

    let mut gated = String::from("state a(true)\npattern P():\n");
    for level in 1..=20 {
        gated.push_str(&format!("{}when a:\n", "    ".repeat(level)));
    }
    gated.push_str(&format!(
        "{}Note\nClip c:\n    for k in 0..100:\n        P\n",
        "    ".repeat(21)
    ));
    with_engine_within(&gated, unbounded, |engine| {
        assert_eq!(engine.analyze(), Ok(()));
        assert_eq!(error_codes(&engine.report.diagnostics), Vec::<Code>::new());
    });
    with_engine_within(&gated, unbounded, |engine| {
        engine.id_space = 1_000;
        assert_eq!(engine.analyze(), Ok(()));
        assert_eq!(
            found(engine),
            vec![(
                Code::ExpansionLimit,
                "the compilation needs more than 1,000 gate conditions, the most its ids number"
                    .to_string()
            )]
        );
    });

    let faulty = "state i(1)\nfn f(x): x + x\npattern P(n):\n    for k in 0..(10 / n):\n        Note\nClip c:\n    P(0)\n    for j in 0..100:\n        Note/(f(i + j))\n";
    let codes_and_bound = |engine: &Engine<'_>| -> (Vec<Code>, bool) {
        let mut codes = error_codes(&engine.report.diagnostics);
        codes.sort_by_key(|code| code.as_str());
        let named = found(engine).iter().any(|(code, message)| {
            *code == Code::ExpansionLimit && message.contains("more than 300 ")
        });
        (codes, named)
    };
    let mut expected = vec![Code::MissingArgument, Code::ExpansionLimit];
    expected.sort_by_key(|code| code.as_str());
    with_engine_within(faulty, unbounded, |engine| {
        engine.id_space = 300;
        assert_eq!(engine.analyze(), Ok(()));
        assert_eq!(codes_and_bound(engine), (expected.clone(), true));
    });
    let fifty_steps = Limits {
        expansion_steps: 50,
        ..unbounded
    };
    with_engine_within(faulty, fifty_steps, |engine| {
        assert_eq!(engine.analyze(), Ok(()));
        assert_eq!(codes_and_bound(engine).0, expected);
    });

    let cut_flow = "Session s\nlet c = Square\nSynth y:\n    Osc | wave=c ** 2bars\nstate i(1)\nfn f(x): x + x\nClip k:\n    for j in 0..100:\n        Note/(f(i + j))\n";
    with_engine_within(cut_flow, unbounded, |engine| {
        assert_eq!(engine.analyze(), Ok(()));
        assert_eq!(
            error_codes(&engine.report.diagnostics),
            vec![Code::TypeMismatch]
        );
    });
    for space in 200..=500 {
        with_engine_within(cut_flow, unbounded, |engine| {
            engine.id_space = space;
            assert_eq!(engine.analyze(), Ok(()));
            assert_eq!(
                error_codes(&engine.report.diagnostics),
                vec![Code::ExpansionLimit],
                "{space}"
            );
        });
    }
    let notes = format!("Clip c:\n{}", "    Note/60\n".repeat(50));
    with_engine_within(&notes, unbounded, |engine| {
        engine.id_space = 50;
        assert_eq!(engine.analyze(), Ok(()));
        assert_eq!(
            found(engine),
            vec![(
                Code::ExpansionLimit,
                "the expansion yields more than 50 instances and wires".to_string()
            )]
        );
    });

    with_engine_within("Session s\n", unbounded, |engine| {
        let scope = engine.source_global;
        engine.id_space = 2;
        for _ in 0..3 {
            engine
                .count_item(Loc::default(), scope)
                .expect("counting meets no internal error");
        }
        let pushed: Vec<bool> = (0..3)
            .map(|_| {
                engine
                    .push_node(Op::Bool(true), Loc::default(), scope)
                    .is_ok()
            })
            .collect();
        assert_eq!(pushed, vec![true, true, false]);
        engine.flush_diagnostics();
        assert_eq!(
            found(engine),
            vec![(
                Code::ExpansionLimit,
                "the expansion yields more than 2 instances and wires".to_string()
            )]
        );
    });

    let warned = "Session s\nfn r(n): if n < 1 then 0 else r(n - 1)\nClip c:\n    Note/(r(4))\nstate n(1)\nSynth y:\n    Osc | freq=n\n";
    let unreached = Code::UnreachableInstance;
    for (calls, expected) in [
        (4, vec![Code::RecursionLimit]),
        (5, vec![Code::IntegerCell, unreached, unreached]),
    ] {
        let limits = Limits {
            recursive_calls: calls,
            ..Limits::default()
        };
        with_engine_within(warned, limits, |engine| {
            assert_eq!(engine.analyze(), Ok(()));
            let codes: Vec<Code> = found(engine).into_iter().map(|(code, _)| code).collect();
            assert_eq!(codes, expected, "{calls}");
        });
    }
    let stopped_early = [
        (
            Limits {
                expanded_items: 4,
                ..Limits::default()
            },
            "Session s\nlet a = Sine\nKit k:\n    Sampler | file=\"{a}\"\nSynth y:\n    Osc | wave=a\n",
        ),
        (
            Limits {
                inlined_calls: 0,
                ..Limits::default()
            },
            "Session s\nlet a = Sine\nfn g(x): x\nSynth y:\n    Osc | wave=g(a)\n",
        ),
    ];
    for (limits, text) in stopped_early {
        let before_infer = with_engine_within(text, limits, |engine| {
            engine.expand().expect("expansion meets no internal error");
            engine
                .count_inlining()
                .expect("the inline count meets no internal error");
            (engine.nodes.len(), engine.scopes.len())
        });
        with_engine_within(text, limits, |engine| {
            assert_eq!(engine.analyze(), Ok(()));
            assert_eq!(
                (engine.nodes.len(), engine.scopes.len()),
                before_infer,
                "{text}"
            );
        });
    }

    let spaces = ["scopes", "values", "cells"];
    for space in spaces {
        for room in [1, 0] {
            with_engine_within("Session s\n", unbounded, |engine| {
                let scope = engine.source_global;
                let taken = match space {
                    "scopes" => engine.scopes.len(),
                    "values" => engine.nodes.len(),
                    _ => engine.bindings.len(),
                };
                engine.id_space = u64::try_from(taken).expect("the arena is small") + room;
                let claimed = match space {
                    "scopes" => engine
                        .scope(
                            ScopeKind::Global { prelude: false },
                            Some(scope),
                            Chain::default(),
                        )
                        .map(|_| ()),
                    "values" => engine
                        .push_node(Op::Bool(true), Loc::default(), scope)
                        .map(|_| ()),
                    _ => engine
                        .push_binding(Binding {
                            kind: BindingKind::Let,
                            init: NodeId(0),
                            annotation: None,
                            name: "x".into(),
                            loc: Loc::default(),
                        })
                        .map(|_| ()),
                };
                engine.flush_diagnostics();
                if room == 1 {
                    assert_eq!(claimed, Ok(()), "{space}");
                    assert_eq!(found(engine), vec![], "{space}");
                } else {
                    assert_eq!(claimed, Err(Internal::missed("id")), "{space}");
                    assert_eq!(
                        found(engine),
                        vec![(
                            Code::ExpansionLimit,
                            format!(
                                "the compilation needs more than {taken} {space}, the most its ids number"
                            )
                        )],
                        "{space}"
                    );
                }
            });
        }
    }

    with_engine_within("Session s\n", unbounded, |engine| {
        let scope = engine.source_global;
        engine.id_space = 2;
        assert_eq!(engine.count_item(Loc::default(), scope), Ok(true));
        assert_eq!(engine.count_item(Loc::default(), scope), Ok(true));
        assert_eq!(engine.count_item(Loc::default(), scope), Ok(false));
        engine.flush_diagnostics();
        assert_eq!(
            found(engine),
            vec![(
                Code::ExpansionLimit,
                "the expansion yields more than 2 instances and wires".to_string()
            )]
        );
    });
    let unbounded_steps = Limits {
        expansion_steps: u64::MAX,
        ..unbounded
    };
    with_engine_within("Session s\n", unbounded_steps, |engine| {
        let scope = engine.source_global;
        engine.id_space = 2;
        assert_eq!(engine.count_step(Loc::default(), scope), Ok(true));
        assert_eq!(engine.count_step(Loc::default(), scope), Ok(true));
        assert_eq!(engine.count_step(Loc::default(), scope), Ok(false));
        engine.flush_diagnostics();
        assert_eq!(
            found(engine),
            vec![(
                Code::ExpansionLimit,
                "the expansion runs more than 2 `for` iterations and pattern uses".to_string()
            )]
        );
    });
}

#[test]
fn a_type_check_that_meets_a_node_with_no_signature_is_an_internal_error() {
    with_engine("state i(1)\nClip c:\n    Note/(i + 1)\n", |engine| {
        assert_eq!(engine.analyze(), Ok(()));
        let operand = engine
            .nodes
            .iter()
            .find_map(|node| {
                if let Op::Binary(symphonyscript_core::BinaryOp::Add, _, operand) = node.op {
                    Some(operand)
                } else {
                    None
                }
            })
            .expect("the program holds the sum");
        engine.memo.sigs.remove(&operand);
        engine.typing = super::infer::Typing::default();
        assert_eq!(engine.infer(), Err(Internal::missed("signature")));
    });
}

#[test]
fn a_cycle_walk_that_misses_is_an_internal_error() {
    with_engine("Session s\n", |engine| {
        engine.run_query = |_, query, need| {
            if query == Query::Sig(NodeId(0)) {
                need.push(Query::Sig(NodeId(1)));
            } else {
                need.push(Query::Sig(NodeId(0)));
            }
            Ok(false)
        };
        assert_eq!(
            engine.demand(Query::Sig(NodeId(0))),
            Err(Internal::missed("cycle declaration"))
        );
        let dep = Query::Sig(NodeId(1));
        assert_eq!(
            engine.cycle(&[(Query::Sig(NodeId(0)), None)], 0, dep),
            Err(Internal::missed("cycle dependency"))
        );
        assert_eq!(
            engine.cycle(&[], 0, dep),
            Err(Internal::missed("demand stack entry"))
        );
    });
}

#[test]
fn a_miss_in_collected_or_derived_state_is_an_internal_error() {
    let missed = |what| Internal::missed(what);
    assert_eq!(
        crate::semantics::collection::strongly_connected(&[vec![1]]),
        Err(missed("graph node"))
    );
    assert_eq!(
        crate::semantics::collection::strongly_connected(&[vec![0], vec![0, 1]]),
        Ok(vec![vec![0], vec![1]])
    );
    let example = vocabulary();
    let wave = example
        .enum_at(0)
        .expect("the example vocabulary declares Wave");
    let empty = Vocabulary::builder("empty")
        .build()
        .expect("the empty vocabulary builds");
    let variants_of = |vocabulary: &Vocabulary| {
        Variants::of(vocabulary).expect("the vocabulary's variants index")
    };
    assert_eq!(
        variants_of(&empty).position(wave, "Sine"),
        Err(missed("enum"))
    );
    assert_eq!(variants_of(&example).position(wave, "Saw"), Ok(Some(1)));
    let pair = Val::array(
        super::shape::Shape::from_lengths(&[2]),
        vec![NodeId(0), NodeId(1)],
    );
    assert_eq!(pair.slice(1), Ok(Val::Scalar(NodeId(1))));
    assert_eq!(pair.slice(2), Err(missed("array element")));
    assert_eq!(
        Val::Scalar(NodeId(0)).slice(0),
        Err(missed("array element"))
    );
    let literal = |digits: &str| crate::semantics::ir::FloatLit {
        negative: false,
        digits: digits.into(),
        exponent: 0,
    };
    assert_eq!(
        literal("3").scaled(0.5).and_then(|half| half.to_f64()),
        Ok(1.5)
    );
    assert_eq!(literal("1x").to_f64(), Err(missed("float literal")));
    assert_eq!(literal("1x").to_f32(), Err(missed("float literal")));
    assert_eq!(
        literal("1x").scaled(2.0).map(|_| ()),
        Err(missed("float literal"))
    );
    with_engine(
        "fn f(x = 1): x\npattern P():\n    Note\nClip c:\n    P\n",
        |engine| {
            assert_eq!(engine.analyze(), Ok(()));
            let inline = &engine.collected.inline;
            let (function, other) = (crate::semantics::ir::FnId(0), crate::semantics::ir::FnId(9));
            assert_eq!(inline.call(function, &[0]), Ok(1));
            assert_eq!(inline.call(other, &[]), Err(missed("function")));
            assert_eq!(
                inline.call(function, &[3]),
                Err(missed("parameter default"))
            );
            let (pattern, unknown) = (
                crate::semantics::ir::PatternId(0),
                crate::semantics::ir::PatternId(9),
            );
            assert_eq!(engine.is_cyclic(pattern), Ok(false));
            assert_eq!(engine.is_cyclic(unknown), Err(missed("pattern")));
            assert_eq!(engine.stand_in(unknown), Err(missed("pattern")));
            assert_eq!(engine.conditions_of(None), Ok(Vec::new()));
            assert_eq!(
                engine.conditions_of(Some(super::ids::ConditionsId(999))),
                Err(missed("condition"))
            );
            let global = engine.source_global;
            assert_eq!(
                engine.demand(Query::Param(global, 0)),
                Err(missed("locals scope"))
            );
            assert_eq!(
                engine.demand(Query::Let(global, super::let_ref::LetRef::Local(0))),
                Err(missed("locals scope"))
            );
        },
    );
}

#[derive(Clone, Copy)]
enum Stage {
    Expand,
    Infer,
    Check,
    CheckWithoutDestinations,
}

fn staged(
    vocabulary: &Vocabulary,
    swapped: &Vocabulary,
    text: &str,
    stage: Stage,
) -> Result<(), Internal> {
    let variants = Variants::of(vocabulary).expect("the vocabulary's variants index");
    let hir = built(vocabulary, text);
    let mut report = Report::default();
    let collected = collect(&hir, vocabulary, &variants, &mut report)?;
    let mut engine = Engine::new(&hir, vocabulary, &collected, &variants, Limits::default());
    match stage {
        Stage::Expand => {
            engine.vocabulary = swapped;
            engine.expand()
        }
        Stage::Infer => {
            engine.expand()?;
            engine.vocabulary = swapped;
            engine.infer()
        }
        Stage::Check => {
            engine.expand()?;
            engine.infer()?;
            engine.vocabulary = swapped;
            engine.check_program()
        }
        Stage::CheckWithoutDestinations => {
            engine.expand()?;
            engine.infer()?;
            engine.memo.destinations.clear();
            engine.check_program()
        }
    }
}

#[test]
fn a_range_bound_that_is_an_array_draws_range_bound() {
    let rejected = [
        (
            format!("{CHORD_FN}Clip c:\n    for i in 0..chord(1):\n        Note\n"),
            "chord(1):",
        ),
        (
            format!("{CHORD_FN}Clip c:\n    for i in chord(1)..3:\n        Note\n"),
            "chord(1)..",
        ),
        (
            format!("{CHORD_FN}let k = chord(1)\nClip c:\n    for i in 0..k:\n        Note\n"),
            "k:",
        ),
    ];
    for (text, bound) in &rejected {
        let (diagnostics, ()) = analyzed(text, |_| ());
        let at = text.rfind(bound).unwrap_or(usize::MAX);
        assert_eq!(
            located(text, &diagnostics),
            vec![(Code::RangeBound, at, vec![])],
            "{text}"
        );
    }
    let fib = "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)\n";
    let accepted = format!("{fib}Clip c:\n    for i in 0..fib(5):\n        Note\n");
    let (diagnostics, (notes, _)) = analyzed(&accepted, notes_and_pitches);
    assert_eq!(error_codes(&diagnostics), Vec::<Code>::new());
    assert_eq!(notes, 5);
}

#[test]
fn every_evaluation_path_checks_the_shape_of_its_value() {
    let chords = chord_vocabulary();
    let example = vocabulary();
    let held = format!("{CHORD_FN}let v = chord(1)\n");
    let rows = vec![
        (
            &example,
            format!("{held}Clip c:\n    Note/v\n"),
            "pitch",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            &chords,
            format!("{held}Chord a | grid=v\n"),
            "grid",
            vec![Code::TypeMismatch],
            None,
        ),
        (
            &chords,
            format!("{held}Chord a | notes=v\n"),
            "notes",
            vec![],
            Some(CVal::Array(CArray::new(vec![
                CVal::Int(62),
                CVal::Int(65),
                CVal::Int(69),
            ]))),
        ),
        (
            &example,
            format!("{held}Clip c:\n    Note/(v[2])\n"),
            "pitch",
            vec![],
            Some(CVal::Int(69)),
        ),
        (
            &example,
            "let x = 3000000000\nClip c:\n    Note/(5i32 + x)\n".to_string(),
            "pitch",
            vec![Code::DoesNotFit],
            None,
        ),
    ];
    for (vocabulary, text, attr, codes, value) in rows {
        let (diagnostics, found) = analyzed_with(vocabulary, &text, |engine| {
            attr_values(engine, attr, EvalTy::I32)
                .into_iter()
                .find(|value| !value.is_error())
        });
        let at = text.rfind(['v', 'x']).unwrap_or(usize::MAX);
        let expected: Vec<(Code, usize, Vec<usize>)> =
            codes.into_iter().map(|code| (code, at, vec![])).collect();
        assert_eq!(located(&text, &diagnostics), expected, "{text}");
        assert_eq!(found, value, "{text}");
    }
}

#[test]
fn a_recursive_call_settles_to_a_string_through_concatenation() {
    let rows = [("s(n - 1) + \"b\"", "abb"), ("\"b\" + s(n - 1)", "bba")];
    for (step, expected) in rows {
        let text = format!(
            "fn s(n): if n < 1 then \"a\" else {step}\nKit k:\n    Sampler | file=(s(2))\n"
        );
        let (diagnostics, found) =
            analyzed(&text, |engine| attr_values(engine, "file", EvalTy::Str));
        assert_eq!(error_codes(&diagnostics), Vec::<Code>::new(), "{text}");
        assert_eq!(found, vec![CVal::Str(expected.into())], "{text}");
    }
    for text in [
        "Clip c:\n    Note/(1 + true)\n",
        "Kit k:\n    Sampler | file=(\"a\" + 1)\n",
    ] {
        let (diagnostics, ()) = analyzed(text, |_| ());
        assert_eq!(error_codes(&diagnostics), vec![Code::OperandType], "{text}");
    }
}

#[test]
fn a_recursive_argument_reads_a_settled_enum() {
    let typed = "Session s\nlet x = Sine\nSynth s2:\n    Osc | wave=x\n";
    let tested = "fn r(n, w): if n < 1 then (if w is Saw then 1 else 2) else r(n - 1, w)\n";
    let returned = "fn r(n, w): if n < 1 then w else r(n - 1, w)\n";
    let first = "fn r(w, n): if n < 1 then w else r(w, n - 1)\n";
    let mut failures = Vec::new();
    for (function, call, attr, ty) in [
        (
            tested,
            "Clip c:\n    Note/(r(2, {}))\n",
            "pitch",
            EvalTy::I32,
        ),
        (
            returned,
            "Synth s3:\n    Osc | wave=(r(2, {}))\n",
            "wave",
            EvalTy::Default,
        ),
        (
            first,
            "Synth s3:\n    Osc | wave=(r({}, 2))\n",
            "wave",
            EvalTy::Default,
        ),
    ] {
        let probe = |argument: &str| {
            let text = format!("{typed}{function}{}", call.replace("{}", argument));
            analyzed(&text, |engine| {
                let ty = match (attr, ty) {
                    ("wave", _) => engine
                        .memo
                        .evals
                        .keys()
                        .find_map(|(_, ty)| matches!(ty, EvalTy::Enum(_)).then_some(*ty))
                        .unwrap_or(ty),
                    _ => ty,
                };
                (attr_value(engine, attr, ty), engine.memo.calls.len())
            })
        };
        let (diagnostics, (value, calls)) = probe("x");
        let (qualified_diagnostics, (qualified, qualified_calls)) = probe("Wave::Sine");
        let found = (error_codes(&diagnostics), value.clone(), calls);
        let wanted = (
            error_codes(&qualified_diagnostics),
            qualified.clone(),
            qualified_calls,
        );
        if found != wanted || qualified.is_none() {
            failures.push(format!("{function}: found {found:?}, wanted {wanted:?}"));
        }
        let text = format!(
            "{typed}{function}{}{}",
            call.replace("{}", "x"),
            call.replace("{}", "Wave::Sine")
                .replace("s3", "s4")
                .replace("c:", "d:")
        );
        let (diagnostics, shared) = analyzed(&text, |engine| engine.memo.calls.len());
        if error_codes(&diagnostics) != Vec::<Code>::new() || shared != qualified_calls {
            failures.push(format!(
                "{function}: shared {shared} against {qualified_calls}"
            ));
        }
    }
    let untyped = format!("Session s\nlet x = Sine\n{tested}Clip c:\n    Note/(r(2, x))\n");
    let (diagnostics, ()) = analyzed(&untyped, |_| ());
    if located(&untyped, &diagnostics)
        != vec![(
            Code::TypeMismatch,
            untyped
                .find("let x = Sine")
                .map_or(0, |at| at + "let x = ".len()),
            vec![],
        )]
    {
        failures.push(format!("untyped: {:?}", error_codes(&diagnostics)));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_compile_time_index_read_costs_nothing_per_passed_over_element() {
    let size = |length: usize| {
        let table: Vec<String> = (0..length).map(|value| value.to_string()).collect();
        let text = format!(
            "Session s\nlet a = [{}]\nClip c:\n    for i in 0..{length}:\n        Note/(a[i])\n",
            table.join(", ")
        );
        let (diagnostics, size) = analyzed(&text, |engine| {
            engine.nodes.len()
                + engine.scopes.len()
                + engine.memo.evals.len()
                + engine.memo.groups.len()
                + engine.typing.expected_seen.len()
                + engine.typing.checks
        });
        assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
        size
    };
    let (smaller, larger) = (size(2000), size(4000));
    assert!(
        larger * 10 <= smaller * 22,
        "{smaller} and {larger} grow faster than 2.2 per doubling"
    );
}

#[test]
fn a_cycle_of_recursive_functions_settles_its_kinds_in_work_linear_in_its_length() {
    let size = |length: usize| {
        let forwarding: Vec<String> = (1..length)
            .map(|index| format!("fn f{index}(n): f{}(n)", (index + 1) % length))
            .collect();
        let text = format!(
            "Session s\nfn f0(n): if n < 1 then 0 else [f1(n - 1), 2][0]\n{}\nClip c:\n    Note/(f0(2))\n",
            forwarding.join("\n")
        );
        let (diagnostics, size) = analyzed(&text, |engine| {
            engine.nodes.len()
                + engine
                    .nodes
                    .iter()
                    .map(|node| node.op.operands().len())
                    .sum::<usize>()
        });
        assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
        size
    };
    let (smaller, larger) = (size(8), size(16));
    assert!(
        larger * 10 <= smaller * 22,
        "{smaller} and {larger} grow faster than 2.2 per doubling"
    );
}

#[test]
fn a_whole_read_of_an_array_let_costs_constant_work() {
    let size = |length: usize| {
        let table: Vec<String> = (0..length).map(|value| value.to_string()).collect();
        let text = format!(
            "Session s\nfn f(x, i): x[i]\nlet a = [{}]\nClip c:\n    for i in 0..{length}:\n        Note/(f(a, i))\n",
            table.join(", ")
        );
        let (diagnostics, size) = analyzed(&text, |engine| {
            engine.nodes.len()
                + engine
                    .nodes
                    .iter()
                    .map(|node| node.op.operands().len())
                    .sum::<usize>()
        });
        assert_eq!(errors_of(&diagnostics), Vec::<&Diagnostic>::new());
        size
    };
    let (smaller, larger) = (size(1000), size(2000));
    assert!(
        larger * 10 <= smaller * 22,
        "{smaller} and {larger} grow faster than 2.2 per doubling"
    );
}

fn in_every_order(items: &[&str]) -> Vec<String> {
    in_orders(Orders::Every, items)
}

fn in_orders(orders: Orders, items: &[&str]) -> Vec<String> {
    orders
        .of(items)
        .into_iter()
        .map(|order| format!("{}\n", order.join("\n")))
        .collect()
}

fn codes_beside_the_entry_warning(diagnostics: &[Diagnostic]) -> Vec<Code> {
    let mut codes: Vec<Code> = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .filter(|code| *code != Code::NoEntryInstance)
        .collect();
    codes.sort();
    codes
}

fn input_cells(engine: &Engine<'_>, name: &str) -> (Vec<u32>, Vec<(Scalar, CVal)>) {
    let mut lengths = Vec::new();
    let mut cells = Vec::new();
    for (position, binding) in engine.bindings.iter().enumerate() {
        let BindingKind::Input { input, .. } = binding.kind else {
            continue;
        };
        if binding.name != name {
            continue;
        }
        if let Some(BoundVal::Array(array)) = engine.memo.inputs.get(&input) {
            lengths = array.0.lengths();
        }
        let id = BindingId(u32::try_from(position).unwrap_or(u32::MAX));
        let Some(own) = engine.typing.own.get(&id).copied() else {
            continue;
        };
        let ty = match own {
            Scalar::I64 => EvalTy::I64,
            Scalar::F64 => EvalTy::F64,
            Scalar::U8
            | Scalar::I32
            | Scalar::F32
            | Scalar::Bool
            | Scalar::Enum(_)
            | Scalar::Str => continue,
        };
        if let Some(value) = engine.memo.evals.get(&(binding.init, ty)) {
            cells.push((own, value.clone()));
        }
    }
    (lengths, cells)
}

type InputRow<'a> = (Vec<&'a str>, Vec<u32>, Vec<(Scalar, CVal)>, Vec<Code>);

#[test]
fn an_array_input_has_one_kind_for_all_its_elements() {
    let float = |values: &[f64]| -> Vec<(Scalar, CVal)> {
        values
            .iter()
            .map(|value| (Scalar::F64, CVal::F64(*value)))
            .collect()
    };
    let integer = |values: &[i128]| -> Vec<(Scalar, CVal)> {
        values
            .iter()
            .map(|value| (Scalar::I64, CVal::Int(*value)))
            .collect()
    };
    let gains_read = ["Clip c | gain=(gains[0])", "Clip d | gain=(gains[1])"];
    let rows: Vec<InputRow<'_>> = vec![
        (
            [&["state gains([1, 0.5])"][..], &gains_read].concat(),
            vec![2],
            float(&[1.0, 0.5]),
            vec![],
        ),
        (
            [&["state gains([0.5, 1])"][..], &gains_read].concat(),
            vec![2],
            float(&[0.5, 1.0]),
            vec![],
        ),
        (
            vec!["state gains([1, 0.5])", "Clip c:\n    Note/(gains[0])"],
            vec![2],
            float(&[1.0, 0.5]),
            vec![Code::FloatToInteger],
        ),
        (
            vec!["state m([[1, 2], [3, 0.5]])"],
            vec![2, 2],
            float(&[1.0, 2.0, 3.0, 0.5]),
            vec![],
        ),
        (
            vec!["state k([1, Wave::Saw])"],
            vec![],
            vec![],
            vec![Code::TypeMismatch],
        ),
        (
            vec!["state k([1, 2]): f64[2]"],
            vec![2],
            float(&[1.0, 2.0]),
            vec![],
        ),
        (
            vec![
                "state pitches([60, 62])",
                "Clip c:\n    Note/(pitches[0])",
                "Osc o | freq=(pitches[1])",
            ],
            vec![2],
            integer(&[60, 62]),
            vec![],
        ),
        (
            [&["state gains([1, 2])"][..], &gains_read].concat(),
            vec![2],
            integer(&[1, 2]),
            vec![Code::IntegerCell],
        ),
    ];
    let mut failures = Vec::new();
    for (items, lengths, cells, codes) in rows {
        let name = items[0]
            .trim_start_matches("state ")
            .split([' ', ':', '('])
            .next()
            .unwrap_or_default()
            .to_string();
        for text in in_every_order(&items) {
            let (diagnostics, found) = analyzed(&text, |engine| input_cells(engine, &name));
            let found_codes = codes_beside_the_entry_warning(&diagnostics);
            if found_codes != codes || found != (lengths.clone(), cells.clone()) {
                failures.push(format!("{text:?}: {found:?} with {found_codes:?}"));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_value_of_fixed_or_widened_type_keeps_its_ieee_value_where_it_is_used() {
    let nan = |value: &CVal| match value {
        CVal::F32(value) => value.is_nan(),
        CVal::F64(value) => value.is_nan(),
        _ => false,
    };
    let accepted: Vec<(Vec<&str>, &str, EvalTy, CVal)> = vec![
        (
            vec!["Osc o | freq=(1e300f64 * 1e300f64)"],
            "freq",
            EvalTy::F64,
            CVal::F64(f64::INFINITY),
        ),
        (
            vec!["Clip c | gain=(3e38f32 * 2.0)"],
            "gain",
            EvalTy::F32,
            CVal::F32(f32::INFINITY),
        ),
        (
            vec!["Clip c | gain=(f32(1e300f64))"],
            "gain",
            EvalTy::F32,
            CVal::F32(f32::INFINITY),
        ),
        (
            vec!["Clip c | gain=(f32(0.0f64 / 0.0f64))"],
            "gain",
            EvalTy::F32,
            CVal::F32(f32::NAN),
        ),
        (
            vec!["Clip c:\n    Note/(i32(1e300f64))"],
            "pitch",
            EvalTy::I32,
            CVal::Int(2147483647),
        ),
        (
            vec!["Clip c:\n    Note/(i32(0.0f64 / 0.0f64))"],
            "pitch",
            EvalTy::I32,
            CVal::Int(0),
        ),
        (
            vec!["let x: f64 = 1e300f64 * 1e300f64", "Osc o | freq=x"],
            "freq",
            EvalTy::F64,
            CVal::F64(f64::INFINITY),
        ),
    ];
    let mut failures = Vec::new();
    for (items, attr, ty, expected) in accepted {
        for text in in_every_order(&items) {
            let (diagnostics, found) = analyzed(&text, |engine| attr_value(engine, attr, ty));
            let errors = error_codes(&diagnostics);
            let holds = match &found {
                Some(found) if nan(&expected) => nan(found),
                Some(found) => *found == expected,
                None => false,
            };
            if !errors.is_empty() || !holds {
                failures.push(format!("{text:?}: {found:?} with {errors:?}"));
                break;
            }
        }
    }
    let rejected: [&[&str]; 4] = [
        &["Osc o | freq=(0.0 / 0.0)"],
        &["Osc o | freq=(1e300 * 1e300)"],
        &["state x(1e300)", "Clip c | gain=x"],
        &["let x: f64 = 1e300 * 1e300", "Osc o | freq=x"],
    ];
    for items in rejected {
        for text in in_every_order(items) {
            let (diagnostics, ()) = analyzed(&text, |_| ());
            if error_codes(&diagnostics) != [Code::DoesNotFit] {
                failures.push(format!("{text:?}: {:?}", error_codes(&diagnostics)));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn a_recursive_call_reuses_only_a_finished_calls_value_in(orders: Orders) {
    let pitches =
        |values: &[i128]| -> Vec<CVal> { values.iter().map(|value| CVal::Int(*value)).collect() };
    let rows: Vec<(Vec<&str>, Vec<CVal>)> = vec![
        (
            vec![
                "fn f(n): if n < 1 then 0 else f(n - 1)",
                "fn g(n): if n < 1 then f(2) else g(n - 1) + f(2)",
                "let a = f(3)",
                "let b = f(3)",
                "Clip c:\n    Note/(a + b + g(2))",
            ],
            pitches(&[0]),
        ),
        (
            vec![
                "fn even(n): if n < 1 then true else odd(n - 1)",
                "fn odd(n): if n < 1 then false else even(n - 1)",
                "Clip c:\n    Note/(if even(10) then 1 else 2)",
            ],
            pitches(&[1]),
        ),
        (
            vec![
                "fn z(): 5",
                "fn two(): z() + z()",
                "Clip c:\n    Note/(two())",
            ],
            pitches(&[10]),
        ),
        (
            vec![
                "fn fib(n): if n < 2 then n else fib(n - 1) + fib(n - 2)",
                "let k = fib(5)",
                "let k2 = fib(6)",
                "fn g(n): if n < 1 then k else g(n - 1) + h(n)",
                "fn h(m): if m < 1 then k2 else h(m - 1)",
                "Clip c:\n    Note/(g(2))\n    Note/(h(1))",
            ],
            pitches(&[21, 8]),
        ),
    ];
    let mut failures = Vec::new();
    for (items, expected) in rows {
        for text in in_orders(orders, &items) {
            let (diagnostics, (found, holes)) = analyzed(&text, |engine| {
                (
                    attr_values(engine, "pitch", EvalTy::I32),
                    placeholders(engine),
                )
            });
            if !errors_of(&diagnostics).is_empty() || found != expected || !holes.is_empty() {
                failures.push(format!(
                    "{text:?}: {found:?} {holes:?} with {:?}",
                    error_codes(&diagnostics)
                ));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_recursive_call_reuses_only_a_finished_calls_value() {
    a_recursive_call_reuses_only_a_finished_calls_value_in(Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_recursive_call_reuses_only_a_finished_calls_value() {
    a_recursive_call_reuses_only_a_finished_calls_value_in(Orders::Every);
}

fn a_recursive_call_with_a_placeholder_argument_is_a_placeholder_in(orders: Orders) {
    let functions = [
        "fn q(n): if n < 1 then 0 else q(n - 1)",
        "fn r(n, m = 2): if n < 1 then m else r(n - 1, m)",
    ];
    let pattern = "pattern P(n):\n    for i in 0..n:\n        Note";
    let positions: [(&[&str], &str); 6] = [
        (&[], "Clip c:\n    for i in 0..X:\n        Note"),
        (&[], "Clip c:\n    for i in X..3:\n        Note"),
        (&[], "Clip c:\n    for i in [X]:\n        Note"),
        (&[], "Clip c:\n    [for i in 0..X: Note]"),
        (&[pattern], "Clip c:\n    P | n=X"),
        (&[], "state zz([1, 2, 3][X])"),
    ];
    let placeholders_alone = [
        ("q(nope)", Code::UnknownName, "nope"),
        ("q(1 +)", Code::ExpressionSyntax, ")"),
        ("q(to.gain)", Code::MisplacedTo, "to"),
        ("q(q(nope))", Code::UnknownName, "nope"),
        ("r(n=nope)", Code::UnknownName, "nope"),
        ("r(z=1)", Code::UnknownParameter, "z"),
    ];
    let counts_and_values: [(&str, [usize; 5], i128); 3] = [
        ("q(2)", [0, 3, 1, 0, 0], 1),
        ("r(1)", [2, 1, 1, 2, 2], 3),
        ("r(n=1)", [2, 1, 1, 2, 2], 3),
    ];
    let mut failures = Vec::new();
    for (position, (extra, line)) in positions.iter().enumerate() {
        let rows = placeholders_alone
            .iter()
            .map(|(argument, code, at)| (*argument, Some((*code, *at)), None))
            .chain(counts_and_values.iter().map(|(argument, counts, value)| {
                (
                    *argument,
                    None,
                    Some((counts.get(position).copied(), *value)),
                )
            }));
        for (argument, fault, kept) in rows {
            let placed = line.replace('X', argument);
            let empty_bundle = "[for i in 0..q(2): Note]";
            let fault = if placed.ends_with(empty_bundle) {
                Some((Code::EmptyBundle, empty_bundle))
            } else {
                fault
            };
            let items: Vec<&str> = functions
                .iter()
                .chain(extra.iter())
                .copied()
                .chain([placed.as_str()])
                .collect();
            for text in in_orders(orders, &items) {
                let (diagnostics, (notes, cells)) = analyzed(&text, |engine| {
                    (notes_and_pitches(engine).0, input_cells(engine, "zz").1)
                });
                let errors: Vec<(Code, &str)> = errors_of(&diagnostics)
                    .iter()
                    .map(|diagnostic| {
                        let range = &diagnostic.span.range;
                        let at = usize::try_from(range.start).unwrap_or(0)
                            ..usize::try_from(range.end).unwrap_or(0);
                        (diagnostic.code, text.get(at).unwrap_or(""))
                    })
                    .collect();
                let holds = match (fault, kept) {
                    (Some(fault), _) => errors == [fault],
                    (None, Some((Some(count), _))) => errors.is_empty() && notes == count,
                    (None, Some((None, value))) => {
                        errors.is_empty() && cells == [(Scalar::I64, CVal::Int(value))]
                    }
                    (None, None) => false,
                };
                if !holds {
                    failures.push(format!("{text:?}: {errors:?}, {notes} notes, {cells:?}"));
                    break;
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_recursive_call_with_a_placeholder_argument_is_a_placeholder() {
    a_recursive_call_with_a_placeholder_argument_is_a_placeholder_in(Orders::ForwardAndReversed);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_a_recursive_call_with_a_placeholder_argument_is_a_placeholder() {
    a_recursive_call_with_a_placeholder_argument_is_a_placeholder_in(Orders::Every);
}

fn errors_at(text: &str, diagnostics: &[Diagnostic]) -> Vec<(Code, String)> {
    errors_of(diagnostics)
        .iter()
        .map(|diagnostic| {
            let range = &diagnostic.span.range;
            let at =
                usize::try_from(range.start).unwrap_or(0)..usize::try_from(range.end).unwrap_or(0);
            (diagnostic.code, text.get(at).unwrap_or("").to_string())
        })
        .collect()
}

type NamedRow<'a> = (Vec<&'a str>, Vec<(Code, &'a str)>, Vec<CVal>);

fn named_rows_in_every_order(rows: Vec<NamedRow<'_>>) -> Vec<String> {
    let mut failures = Vec::new();
    for (items, wanted, velocities) in rows {
        let wanted: Vec<(Code, String)> = wanted
            .iter()
            .map(|(code, at)| (*code, (*at).to_string()))
            .collect();
        for text in in_every_order(&items) {
            let (diagnostics, (found, holes)) = analyzed(&text, |engine| {
                (
                    attr_values(engine, "velocity", EvalTy::F32),
                    placeholders(engine),
                )
            });
            let errors = errors_at(&text, &diagnostics);
            let clean_holds = !wanted.is_empty() || (holes.is_empty() && found == velocities);
            if errors != wanted || !clean_holds {
                failures.push(format!("{text:?}: {errors:?} {found:?} {holes:?}"));
                break;
            }
        }
    }
    failures
}

#[test]
fn a_stand_in_placements_own_name_names_nothing_in_its_pattern() {
    let velocities = vec![CVal::F32(0.5), CVal::F32(0.5)];
    let rows: Vec<NamedRow<'_>> = vec![
        (
            vec![
                "pattern Loop():\n    Clip as k:\n        Plays: k",
                "Loop top",
                "Session s",
            ],
            vec![(Code::UnknownName, "k")],
            vec![],
        ),
        (
            vec![
                "pattern P():\n    Note as k | pitch=k.velocity",
                "Clip c:\n    P",
            ],
            vec![(Code::UnknownName, "k")],
            vec![],
        ),
        (
            vec![
                "pattern P():\n    Clip as k | gain=k.nothing:\n        Note",
                "P top",
            ],
            vec![(Code::UnknownName, "k")],
            vec![],
        ),
        (
            vec![
                "pattern Loop(default root, again):\n    Clip:\n        Note/root Note/(root + 7)\n        Plays: again",
                "Loop low /36 | again=low",
            ],
            vec![],
            vec![],
        ),
        (
            vec![
                "pattern R():\n    Clip as k:\n        Note",
                "R top",
                "Session s",
            ],
            vec![],
            vec![],
        ),
        (
            vec![
                "pattern P():\n    Note as n | velocity=0.5\n    Note | velocity=n.velocity",
                "Clip c:\n    P",
            ],
            vec![],
            velocities,
        ),
    ];
    let failures = named_rows_in_every_order(rows);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_patterns_lets_see_the_names_of_its_body() {
    let rows: Vec<NamedRow<'_>> = vec![
        (
            vec![
                "pattern P():\n    let v = n.velocity\n    Note as n | velocity=0.5\n    Note | velocity=v",
                "Clip c:\n    P",
            ],
            vec![],
            vec![CVal::F32(0.5), CVal::F32(0.5)],
        ),
        (
            vec![
                "pattern P():\n    let a = n.velocity\n    Note as n | velocity=a\n    Note",
                "Clip c:\n    P",
            ],
            vec![(Code::DeclarationCycle, "a")],
            vec![],
        ),
        (
            vec![
                "pattern P():\n    let v = n.velocity\n    Note as n | velocity=0.5",
                "Clip c:\n    P",
            ],
            vec![(Code::UnknownName, "n")],
            vec![],
        ),
        (
            vec!["pattern P():\n    let a = nope", "Clip c:\n    P"],
            vec![(Code::UnknownName, "nope"), (Code::PlacementSyntax, "")],
            vec![],
        ),
        (
            vec!["pattern P():\n    let a = nope"],
            vec![(Code::UnknownName, "nope"), (Code::PlacementSyntax, "")],
            vec![],
        ),
        (
            vec![
                "pattern P():\n    let a = k.velocity\n    = Note as k\n    Note",
                "Clip c:\n    P",
            ],
            vec![(Code::PlacementSyntax, "=")],
            vec![],
        ),
    ];
    let mut failures = named_rows_in_every_order(rows);
    let cycle = "pattern P():\n    let a = n.velocity\n    Note as n | velocity=a\n    Note\nClip c:\n    P\n";
    let (diagnostics, ()) = analyzed(cycle, |_| ());
    let names: Vec<&str> = errors_of(&diagnostics)
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::DeclarationCycle)
        .flat_map(|diagnostic| {
            let chain = diagnostic
                .summary
                .rsplit_once(": ")
                .map_or("", |(_, chain)| chain);
            chain.split(" → ")
        })
        .collect();
    if names != ["a", "n.velocity", "a"] {
        failures.push(format!("{cycle:?} names {names:?}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[path = "../../../tests/robustness_corpus/mod.rs"]
mod robustness_corpus;

fn kinded_tokens(text: &str, vocabulary: &Vocabulary) -> Vec<((usize, usize), usize)> {
    let source = Source {
        path: "mutant.sym".into(),
        text: text.into(),
    };
    crate::syntax::lex(&source, vocabulary)
        .tokens
        .iter()
        .filter_map(|token| {
            let start = usize::try_from(token.range.start).ok()?;
            let end = usize::try_from(token.range.end).ok()?;
            Some(((start, end), token.kind as usize))
        })
        .collect()
}

fn clean_mutant_failure(compiler: &Compiler, mutant: &str) -> Option<Option<String>> {
    let sources = [Source {
        path: "mutant.sym".into(),
        text: mutant.to_string(),
    }];
    let (diagnostics, found) = compiler.analyzed(&sources, placeholders);
    let found = found?;
    if !errors_of(&diagnostics).is_empty() {
        return None;
    }
    Some((!found.is_empty()).then(|| format!("{mutant:?}: {found:?}")))
}

fn mutants_draw_no_placeholder(shard: usize) {
    let mut failures = Vec::new();
    let mut clean = 0;
    let mut index = 0;
    for (vocabulary, texts) in robustness_corpus::corpora() {
        let compiler = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        for text in &texts {
            let tokens: Vec<(usize, usize)> = kinded_tokens(text, &vocabulary)
                .into_iter()
                .map(|(span, _)| span)
                .collect();
            for mutant in robustness_corpus::mutants(text, &tokens) {
                index += 1;
                if index % MUTANT_SHARDS != shard {
                    continue;
                }
                let Some(failure) = clean_mutant_failure(&compiler, &mutant) else {
                    continue;
                };
                clean += 1;
                failures.extend(failure);
            }
        }
    }
    assert!(
        clean > 1_000 / MUTANT_SHARDS,
        "{clean} mutants drew no error"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn no_mutant_of_an_example_or_fixture_that_draws_no_error_holds_a_placeholder() {
    let corpora = robustness_corpus::corpora();
    let (failures, clean) = covered(|rows| {
        let mut failures = Vec::new();
        let mut clean = 0;
        for (corpus, (vocabulary, texts)) in corpora.iter().enumerate() {
            let compiler = Compiler::new(vocabulary).expect("the vocabulary compiles");
            for text in texts {
                for (span, kind) in kinded_tokens(text, vocabulary) {
                    for mutation in 0..robustness_corpus::MUTATIONS {
                        if !rows.take(&[corpus, kind, mutation]) {
                            continue;
                        }
                        let Some(mutant) = robustness_corpus::mutant(text, span, mutation) else {
                            continue;
                        };
                        let Some(failure) = clean_mutant_failure(&compiler, &mutant) else {
                            continue;
                        };
                        clean += 1;
                        failures.extend(failure);
                    }
                }
            }
        }
        (failures, clean)
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(clean, 34);
}

const MUTANT_SHARDS: usize = 4;

#[test]
#[ignore = "exhaustive"]
fn exhaustive_no_mutant_of_an_example_or_fixture_that_draws_no_error_holds_a_placeholder_first_quarter()
 {
    mutants_draw_no_placeholder(0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_no_mutant_of_an_example_or_fixture_that_draws_no_error_holds_a_placeholder_second_quarter()
 {
    mutants_draw_no_placeholder(1);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_no_mutant_of_an_example_or_fixture_that_draws_no_error_holds_a_placeholder_third_quarter()
 {
    mutants_draw_no_placeholder(2);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_no_mutant_of_an_example_or_fixture_that_draws_no_error_holds_a_placeholder_fourth_quarter()
 {
    mutants_draw_no_placeholder(3);
}

#[test]
fn the_evaluation_of_expanded_code_reports_what_a_kind_probe_meets() {
    let items = [
        "fn fib(n): if n < 2 then n else fib+(n - 1)",
        "Clip c:
    Note/(fib(3))",
    ];
    for text in in_every_order(&items) {
        let (diagnostics, ()) = analyzed(&text, |_| ());
        assert_eq!(
            errors_at(&text, &diagnostics),
            [(Code::NotAValue, "fib".to_string())],
            "{text:?}"
        );
    }
}
