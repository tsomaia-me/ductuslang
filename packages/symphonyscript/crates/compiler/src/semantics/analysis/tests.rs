use symphonyscript_fixtures::vocabulary;

use super::Engine;
use crate::diagnostics::Severity;
use crate::semantics::collection::collect;
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
