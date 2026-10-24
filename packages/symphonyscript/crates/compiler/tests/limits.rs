mod support;

use std::time::{Duration, Instant};

use symphonyscript::{Code, Compiler, Severity, Source};
use symphonyscript_core::{Count, Vocabulary};

fn codes(text: &str) -> Vec<Code> {
    support::checked(text)
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn a_million_iterations_and_uses_in_all_are_accepted_and_one_more_is_not() {
    let million = "pattern P():\n    for j in 0..0:\n        Note\nClip c:\n    for i in 0..500000:\n        P\n";
    assert_eq!(codes(million), vec![]);
    let one_more =
        format!("{million}    for k in 0..1:\n        for j in 0..0:\n            Note\n");
    assert_eq!(codes(&one_more), vec![Code::ExpansionLimit]);
}

#[test]
fn an_integer_that_would_reach_two_to_the_127_is_reported_before_it_is_computed() {
    let just_under = "170141183460469231731687303715884105727";
    let reference = "Clip c:\n    Note/(2 ** 127)\n";
    assert_eq!(codes(reference), vec![Code::IntegerOverflow]);
    let programs = [
        "Clip c:\n    Note/(2 ** 1000000000)\n".to_string(),
        "let e = 2 ** 2 ** 2 ** 2 ** 2\nClip c:\n    Note/e\n".to_string(),
        format!("Clip c:\n    Note/({just_under} * {just_under})\n"),
    ];
    for text in &programs {
        assert_eq!(codes(text), vec![Code::IntegerOverflow], "{text}");
        let (time, reference_time) = best_times(text, reference);
        assert!(
            time <= reference_time * 10,
            "{time:?} against {reference_time:?}: {text}"
        );
    }
}

fn best_times(text: &str, reference: &str) -> (Duration, Duration) {
    let timed = |text: &str| {
        let start = Instant::now();
        for _ in 0..10 {
            support::checked(text);
        }
        start.elapsed()
    };
    let (mut best, mut best_reference) = (Duration::MAX, Duration::MAX);
    for _ in 0..5 {
        best = best.min(timed(text));
        best_reference = best_reference.min(timed(reference));
    }
    (best, best_reference)
}

#[test]
fn variant_lookups_cost_constant_time_per_name() {
    let compiler_over = |names: Vec<String>| {
        let mut v = Vocabulary::builder("wide");
        let variants = v.enumeration("E", &names);
        let other = v.enumeration("F", ["Other"]);
        let kind = v.node("K");
        v.kind(kind)
            .top_level(Count::Any)
            .attr_enum("e", variants, "V65535")
            .attr_enum("f", other, "Other");
        let vocabulary = v.build().expect("the vocabulary builds");
        Compiler::new(&vocabulary).expect("the vocabulary compiles")
    };
    let named = |range: std::ops::Range<usize>| range.map(|index| format!("V{index}")).collect();
    let wide = compiler_over(named(0..65_536));
    let narrow = compiler_over(named(65_472..65_536));
    let mut text = String::new();
    for index in 0..20_000 {
        text.push_str(&format!("K k{index} | e=V{}\n", 65_472 + index % 64));
    }
    text.push_str("K x | e=Other\n");
    let source = [Source {
        path: "wide.sym".into(),
        text,
    }];
    let errors = |compiler: &Compiler| -> Vec<Code> {
        compiler
            .check(&source)
            .iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
            .map(|diagnostic| diagnostic.code)
            .collect()
    };
    assert_eq!(errors(&wide), vec![Code::UnknownName]);
    assert_eq!(errors(&narrow), vec![Code::UnknownName]);
    let timed = |compiler: &Compiler| {
        let start = Instant::now();
        compiler.check(&source);
        start.elapsed()
    };
    let (mut wide_time, mut narrow_time) = (Duration::MAX, Duration::MAX);
    for _ in 0..3 {
        wide_time = wide_time.min(timed(&wide));
        narrow_time = narrow_time.min(timed(&narrow));
    }
    assert!(
        wide_time <= narrow_time * 3,
        "{wide_time:?} over 65,536 variants against {narrow_time:?} over 64"
    );
}
