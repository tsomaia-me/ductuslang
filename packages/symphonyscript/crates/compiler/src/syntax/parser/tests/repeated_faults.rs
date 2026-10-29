use symphonyscript_core::Vocabulary;

use super::super::Parser;
use crate::diagnostics::Code;
use crate::source::Source;

fn work_and_codes(line: &str, repetitions: usize) -> (usize, Vec<Code>) {
    let source = Source {
        path: "broken.sym".into(),
        text: line.repeat(repetitions),
    };
    let vocabulary = Vocabulary::builder("empty")
        .build()
        .expect("an empty vocabulary is valid");
    let (_, diagnostics, cost) = Parser::new(&source, &vocabulary).run_counting();
    let codes = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    (cost.work, codes)
}

#[test]
#[ignore = "stress"]
fn stress_repeated_faults_cost_work_linear_in_their_number() {
    for (line, repetitions, faults) in [
        ("let a = (1 2\n", 8_000, &[Code::ExpressionSyntax][..]),
        (
            "let a = 1_ +\n",
            20_000,
            &[Code::MisplacedUnderscore, Code::ExpressionSyntax][..],
        ),
    ] {
        let (half_work, half_codes) = work_and_codes(line, repetitions / 2);
        let (work, codes) = work_and_codes(line, repetitions);
        assert_eq!(codes, faults.repeat(repetitions), "{line:?}");
        assert_eq!(half_codes.len() * 2, codes.len(), "{line:?}");
        assert!(
            work * 2 < half_work * 5,
            "{line:?}: {half_work} for {} lines, {work} for {repetitions}",
            repetitions / 2
        );
        assert!(work < line.len() * repetitions * 6, "{line:?}: {work}");
    }
}
