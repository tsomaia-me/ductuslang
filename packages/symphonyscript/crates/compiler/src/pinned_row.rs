use crate::compiler::Compiler;
use crate::diagnostics::Diagnostic;
use crate::source::Source;

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn spanned(file: &str, range: &std::ops::Range<u32>) -> String {
    format!(
        "\"file\": {}, \"span\": [{}, {}]",
        quoted(file),
        range.start,
        range.end
    )
}

pub(crate) fn pinned_row(program: &str, diagnostics: &[Diagnostic]) -> String {
    let drawn: Vec<String> = diagnostics
        .iter()
        .map(|diagnostic| {
            let related: Vec<String> = diagnostic
                .related
                .iter()
                .map(|related| {
                    format!(
                        "{{{}, \"label\": {}}}",
                        spanned(&related.span.file, &related.span.range),
                        quoted(&related.label)
                    )
                })
                .collect();
            format!(
                "{{\"code\": {}, {}, \"summary\": {}, \"help\": {}, \"related\": [{}]}}",
                quoted(diagnostic.code.as_str()),
                spanned(&diagnostic.span.file, &diagnostic.span.range),
                quoted(&diagnostic.summary),
                diagnostic
                    .help
                    .as_deref()
                    .map_or("null".to_string(), quoted),
                related.join(", ")
            )
        })
        .collect();
    format!(
        "pinned-row {{\"program\": {}, \"diagnostics\": [{}]}}",
        quoted(program),
        drawn.join(", ")
    )
}

pub(crate) fn pinned_counts(program: &str, counts: &[(String, Vec<u64>)]) -> String {
    let counted: Vec<String> = counts
        .iter()
        .map(|(counter, counts)| {
            let counts: Vec<String> = counts.iter().map(u64::to_string).collect();
            format!(
                "{{\"counter\": {}, \"counts\": [{}]}}",
                quoted(counter),
                counts.join(", ")
            )
        })
        .collect();
    format!(
        "pinned-row {{\"program\": {}, \"diagnostics\": [{}]}}",
        quoted(program),
        counted.join(", ")
    )
}

pub(crate) fn failed(
    program: &str,
    diagnostics: &[Diagnostic],
    message: impl std::fmt::Display,
) -> String {
    format!("{}\n{message}", pinned_row(program, diagnostics))
}

pub(crate) fn failed_with(
    compiler: &Compiler,
    program: &str,
    message: impl std::fmt::Display,
) -> String {
    let source = Source {
        path: "test.sym".into(),
        text: program.into(),
    };
    failed(program, &compiler.check(&[source]), message)
}

pub(crate) fn failed_counts(
    program: &str,
    counts: &[(String, Vec<u64>)],
    message: impl std::fmt::Display,
) -> String {
    format!("{}\n{message}", pinned_counts(program, counts))
}

pub(crate) fn assert_no_failure(failures: &[String]) {
    for failure in failures {
        println!("{failure}");
    }
    assert!(failures.is_empty(), "{} rows fail", failures.len());
}

pub(crate) fn failed_example(program: &str, message: impl std::fmt::Display) -> String {
    let compiler = Compiler::new(&symphonyscript_fixtures::vocabulary())
        .unwrap_or_else(|failed| panic!("the example vocabulary compiles: {failed:?}"));
    failed_with(&compiler, program, message)
}

pub(crate) fn assert_pinned_failures(failures: &[String], pinned: &str, ledger: u32) {
    let mut found: Vec<&str> = failures
        .iter()
        .map(|failure| failure.lines().next().unwrap_or_default())
        .collect();
    found.sort_unstable();
    let mut expected: Vec<&str> = pinned.lines().filter(|line| !line.is_empty()).collect();
    expected.sort_unstable();
    for line in found.iter().filter(|line| !expected.contains(line)) {
        println!("failing beyond the pin of ledger {ledger}: {line}");
    }
    for line in expected.iter().filter(|line| !found.contains(line)) {
        println!("pinned by ledger {ledger} and no longer failing: {line}");
    }
    assert!(
        found == expected,
        "the failing set differs from the {} rows ledger {ledger} pins: {} fail",
        expected.len(),
        found.len()
    );
}
