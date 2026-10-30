use symphonyscript_fixtures::vocabulary;

use super::{Variants, collect};
use crate::semantics::ir::{Hir, build_file};
use crate::semantics::reporting::Report;
use crate::source::Source;
use crate::syntax::parse;

fn walked(text: &str) -> u64 {
    let vocabulary = vocabulary();
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let mut hir = Hir::default();
    hir.seal_prelude();
    let parsed = parse(&source, &vocabulary);
    build_file(
        &mut hir,
        &source.path,
        &parsed,
        &vocabulary,
        &mut crate::semantics::reporting::Report::default(),
    )
    .expect("the program builds");
    let variants = Variants::of(&vocabulary).expect("the vocabulary's variants index");
    let mut report = Report::default();
    let collected =
        collect(&hir, &vocabulary, &variants, &mut report).expect("the program collects");
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    collected.inline.walked
}

fn caller_and_callee(sites: usize, caller_first: bool) -> String {
    let caller = format!("fn caller(x): {}\n", vec!["big(x)"; sites].join(" + "));
    let callee = format!("fn big(x): {}\n", vec!["x"; sites].join(" + "));
    let (first, second) = if caller_first {
        (caller, callee)
    } else {
        (callee, caller)
    };
    format!("{first}{second}Clip c:\n    Note/1\n")
}

#[test]
#[ignore = "stress"]
fn stress_the_inline_count_walks_each_function_a_bounded_number_of_times() {
    let caller_first = |sites| walked(&caller_and_callee(sites, true));
    let callee_first = |sites| walked(&caller_and_callee(sites, false));
    let (small, middle, large) = (
        caller_first(4_000),
        caller_first(8_000),
        caller_first(16_000),
    );
    assert!(
        large - middle <= 2 * (middle - small),
        "{small}, {middle} and {large} grow faster than the sites"
    );
    let reference = callee_first(16_000);
    assert!(
        large <= 2 * reference,
        "{large} against {reference} in the callee-first order"
    );
}
