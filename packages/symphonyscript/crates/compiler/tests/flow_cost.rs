use std::time::{Duration, Instant};

use symphonyscript::{Code, Compiler, Severity, Source};
use symphonyscript_core::{Count, Vocabulary};

fn vocabulary() -> Vocabulary {
    let mut v = Vocabulary::builder("flow cost");
    let wave = v.enumeration("Wave", ["Sine", "Saw"]);
    let color = v.enumeration("Color", ["Red", "Blue"]);
    let both = v.node("K");
    v.kind(both)
        .top_level(Count::Any)
        .attr_enum("wave", &wave, "Sine")
        .attr_enum("color", &color, "Red");
    v.build().expect("the flow cost vocabulary builds")
}

fn program(uses: usize) -> String {
    let mut text =
        String::from("pattern P():\n    let a = Sine\n    let b = a\n    K | wave=a color=b\n");
    for use_ in 0..uses {
        text.push_str(&format!("P p{use_}\n"));
    }
    text
}

fn checked(compiler: &Compiler, text: &str) -> (Duration, Vec<(Code, String, Vec<String>)>) {
    let source = Source {
        path: "test.sym".into(),
        text: text.into(),
    };
    let span = |range: std::ops::Range<u32>| {
        let at = usize::try_from(range.start).unwrap_or(0)..usize::try_from(range.end).unwrap_or(0);
        text.get(at).unwrap_or("").to_string()
    };
    let start = Instant::now();
    let diagnostics = compiler.check(&[source]);
    let elapsed = start.elapsed();
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity() == Severity::Error)
        .map(|diagnostic| {
            let mut secondary: Vec<String> = diagnostic
                .related
                .iter()
                .map(|related| &related.span)
                .map(|loc| span(loc.range.clone()))
                .collect();
            secondary.sort();
            (
                diagnostic.code,
                span(diagnostic.span.range.clone()),
                secondary,
            )
        })
        .collect();
    (elapsed, errors)
}

#[test]
fn erroneous_flows_report_in_time_linear_in_their_number() {
    let compiler = Compiler::new(&vocabulary()).expect("the vocabulary compiles");
    let fastest = |uses: usize| {
        let text = program(uses);
        let mut best = Duration::MAX;
        for _ in 0..3 {
            let (elapsed, errors) = checked(&compiler, &text);
            assert_eq!(errors.len(), uses, "one diagnostic per use");
            assert_eq!(
                errors.first(),
                Some(&(
                    Code::TypeMismatch,
                    "let a = Sine\n".to_string(),
                    vec!["P p0".to_string(), "let b = a\n".to_string()]
                ))
            );
            best = best.min(elapsed);
        }
        best
    };
    let (smaller, larger) = (fastest(2000), fastest(8000));
    assert!(
        larger < smaller * 8,
        "{smaller:?} for 2,000 uses and {larger:?} for 8,000 grow faster than linearly"
    );
}
