mod robustness_corpus;

use std::panic::{AssertUnwindSafe, catch_unwind};

use robustness_corpus::{corpora, mutants};
use symphonyscript::syntax::lex;
use symphonyscript::{Code, Compiler, Source};
use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::vocabulary;

const SAMPLED_ONE_IN: usize = 3;

fn tokens(text: &str, vocabulary: &Vocabulary) -> Vec<(usize, usize)> {
    let source = Source {
        path: "mutant.sym".into(),
        text: text.into(),
    };
    lex(&source, vocabulary)
        .tokens
        .iter()
        .filter_map(|token| {
            let start = usize::try_from(token.range.start).ok()?;
            let end = usize::try_from(token.range.end).ok()?;
            Some((start, end))
        })
        .collect()
}

#[test]
fn no_mutant_of_an_example_or_fixture_draws_an_internal_error_or_panics() {
    let mut checked = 0;
    let mut index = 0;
    let mut failures = Vec::new();
    for (vocabulary, texts) in corpora() {
        let compiler = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        for mutant in texts
            .iter()
            .flat_map(|text| mutants(text, &tokens(text, &vocabulary)))
        {
            let sampled = index % SAMPLED_ONE_IN == 0;
            index += 1;
            if !sampled {
                continue;
            }
            checked += 1;
            let sources = [Source {
                path: "mutant.sym".into(),
                text: mutant.clone(),
            }];
            match catch_unwind(AssertUnwindSafe(|| compiler.check(&sources))) {
                Ok(diagnostics) => {
                    if diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.code == Code::InternalError)
                    {
                        failures.push(format!("internal-error: {mutant:?}"));
                    }
                }
                Err(_) => failures.push(format!("panic: {mutant:?}")),
            }
        }
    }
    assert!(checked > 5_000, "{checked} mutants checked");
    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn the_absences_that_are_normal_draw_no_internal_error() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let empty: [Source; 0] = [];
    assert!(
        compiler
            .check(&empty)
            .iter()
            .all(|diagnostic| diagnostic.code != Code::InternalError)
    );
    for text in [
        "fn r(n): if n < 1 then [] else r(n - 1)\nClip c:\n    Note/(r(2)[0])\n",
        "fn r(a, n): if n < 1 then 0 else r(a, n - 1)\nClip c:\n    Note/(r([], 2))\n",
        "input f = true\nlet x = if f then Sine else Saw\nClip c:\n    Note when x is x\n",
        "Clip c:\n    Note/([][0])\n",
    ] {
        let diagnostics = compiler.check(&[Source {
            path: "test.sym".into(),
            text: text.into(),
        }]);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != Code::InternalError),
            "{text}"
        );
    }
}
