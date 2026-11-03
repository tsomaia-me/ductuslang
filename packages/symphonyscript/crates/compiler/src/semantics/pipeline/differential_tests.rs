use crate::compiler::Compiler;
use crate::selector::Selector;
use crate::syntax::front_end_goldens::{
    Corpus, fixture_corpus, generated_corpus, parser_test_corpus, robustness_quarter, source,
    spec_and_lesson_corpora,
};

type Reviewed = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
);

const REVIEWED: [Reviewed; 2] = [
    (
        "476",
        "let q = 1\nClip k:\n    Note/((true is q)[if true then [1, 2][5] else [1, 2]])\n",
        &[
            ("not-an-array", "true is q"),
            ("operand-type", "true is q"),
            ("type-mismatch", "if true then [1, 2][5] else [1, 2]"),
            ("index-out-of-range", "5"),
        ],
    ),
    (
        "476",
        "Clip k:\n    Note/([1, 2][true..])\n",
        &[("type-mismatch", "[1, 2][true..]"), ("index-type", "true")],
    ),
];

fn spanned(text: &str, found: &[crate::diagnostics::Diagnostic]) -> Vec<(String, String)> {
    let mut spans: Vec<(String, String)> = found
        .iter()
        .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::NoEntryInstance)
        .map(|diagnostic| {
            let range = diagnostic.span.range.start as usize..diagnostic.span.range.end as usize;
            (
                diagnostic.code.to_string(),
                text.get(range).unwrap_or_default().to_string(),
            )
        })
        .collect();
    spans.sort();
    spans
}

fn reviewed(text: &str) -> Option<(&'static str, Vec<(String, String)>)> {
    REVIEWED
        .iter()
        .find(|(_, program, _)| *program == text)
        .map(|(ledger, _, expected)| {
            let mut expected: Vec<(String, String)> = expected
                .iter()
                .map(|(code, span)| ((*code).to_string(), (*span).to_string()))
                .collect();
            expected.sort();
            (*ledger, expected)
        })
}

#[test]
fn the_reviewed_moves_draw_their_expected_sets_on_the_new_core_476() {
    let new =
        Compiler::new(&symphonyscript_fixtures::vocabulary()).expect("the vocabulary compiles");
    for (ledger, text, _) in REVIEWED {
        let found = new
            .checked_by_new_core(&[source(text)])
            .expect("the new core takes the program");
        let expected = reviewed(text).map(|(_, expected)| expected);
        assert_eq!(Some(spanned(text, &found)), expected, "{ledger}: {text}");
    }
}

fn assert_equal(corpora: &[Corpus]) -> u64 {
    let mut taken = 0;
    let mut differing = Vec::new();
    let mut moved = Vec::new();
    for corpus in corpora {
        let mut old = Compiler::new(&corpus.vocabulary).expect("the vocabulary compiles");
        old.select(Selector::OldEngine);
        let new = Compiler::new(&corpus.vocabulary).expect("the vocabulary compiles");
        for text in &corpus.texts {
            let sources = [source(text)];
            let Some(found) = new.checked_by_new_core(&sources) else {
                continue;
            };
            taken += 1;
            let expected = old.check(&sources);
            let without_reactive_lets: Vec<_> = found
                .iter()
                .filter(|diagnostic| diagnostic.code != crate::diagnostics::Code::ReactiveLet)
                .cloned()
                .collect();
            if let Some((ledger, wanted)) = reviewed(text) {
                if spanned(text, &found) == wanted {
                    moved.push(format!("{ledger} {}:\n{text}", corpus.name));
                } else {
                    differing.push(format!(
                        "{ledger} {}:\n{text}\n  new core {found:#?}",
                        corpus.name
                    ));
                }
                continue;
            }
            if found != expected && without_reactive_lets == expected {
                moved.push(format!("464 {}:\n{text}", corpus.name));
                continue;
            }
            if found != expected {
                differing.push(format!(
                    "{}:\n{text}\n  new core {found:#?}\n  old engine {expected:#?}",
                    corpus.name
                ));
            }
        }
    }
    eprintln!("the new core took {taken} programs");
    eprintln!(
        "{} of them move by a reviewed ledger rule (464, 476):\n{}",
        moved.len(),
        moved.join("\n")
    );
    let shown: Vec<&str> = differing.iter().take(5).map(String::as_str).collect();
    assert!(
        differing.is_empty(),
        "{} of {taken} programs the new core took differ from the old engine:\n{}",
        differing.len(),
        shown.join("\n")
    );
    assert!(taken > 0, "the new core took no program");
    taken
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_differential_the_spec_and_lesson_examples() {
    assert_equal(&spec_and_lesson_corpora());
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_differential_the_fixtures_and_the_parser_tests() {
    assert_equal(&[fixture_corpus(), parser_test_corpus(), generated_corpus()]);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_differential_the_robustness_corpus() {
    let corpora: Vec<Corpus> = (0..4).flat_map(robustness_quarter).collect();
    assert_equal(&corpora);
}
