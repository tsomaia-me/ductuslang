mod robustness_corpus;
mod support;

use std::panic::{AssertUnwindSafe, catch_unwind};

use robustness_corpus::{MUTATIONS, corpora, mutant, mutants};
use symphonyscript::syntax::lex;
use symphonyscript::{Code, Compiler, Source};
use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{Rows, covered, vocabulary};

const SAMPLED_ONE_IN: usize = 3;

fn kinded_tokens(text: &str, vocabulary: &Vocabulary) -> Vec<((usize, usize), usize)> {
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
            Some(((start, end), token.kind as usize))
        })
        .collect()
}

fn tokens(text: &str, vocabulary: &Vocabulary) -> Vec<(usize, usize)> {
    kinded_tokens(text, vocabulary)
        .into_iter()
        .map(|(span, _)| span)
        .collect()
}

fn drawn_failure(compiler: &Compiler, mutant: &str) -> Option<String> {
    let sources = [Source {
        path: "mutant.sym".into(),
        text: mutant.to_string(),
    }];
    match catch_unwind(AssertUnwindSafe(|| compiler.check(&sources))) {
        Ok(diagnostics) => diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == Code::InternalError)
            .then(|| format!("internal-error: {mutant:?}")),
        Err(_) => Some(format!("panic: {mutant:?}")),
    }
}

fn covered_mutants_draw_no_internal_error(
    rows: &Rows,
    corpora: &[(Vocabulary, Vec<String>)],
) -> (Vec<String>, usize) {
    let mut failures = Vec::new();
    let mut checked = 0;
    for (corpus, (vocabulary, texts)) in corpora.iter().enumerate() {
        let compiler = Compiler::new(vocabulary).expect("the vocabulary compiles");
        for text in texts {
            for (span, kind) in kinded_tokens(text, vocabulary) {
                for mutation in 0..MUTATIONS {
                    if !rows.take(&[corpus, kind, mutation]) {
                        continue;
                    }
                    let Some(mutant) = mutant(text, span, mutation) else {
                        continue;
                    };
                    checked += 1;
                    failures.extend(drawn_failure(&compiler, &mutant));
                }
            }
        }
    }
    (failures, checked)
}

#[test]
fn no_mutant_of_an_example_or_fixture_draws_an_internal_error_or_panics() {
    let corpora = corpora();
    let (failures, checked) =
        covered(|rows| covered_mutants_draw_no_internal_error(rows, &corpora));
    assert_eq!(failures, Vec::<String>::new());
    assert_eq!(checked, 1_286);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_no_mutant_of_an_example_or_fixture_draws_an_internal_error_or_panics_in_the_odd_samples()
 {
    mutants_draw_no_internal_error(0);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_no_mutant_of_an_example_or_fixture_draws_an_internal_error_or_panics_in_the_even_samples()
 {
    mutants_draw_no_internal_error(1);
}

fn mutants_draw_no_internal_error(half: usize) {
    let mut checked = 0;
    let mut index = 0;
    let mut failures = Vec::new();
    for (vocabulary, texts) in corpora() {
        let compiler = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        for mutant in texts
            .iter()
            .flat_map(|text| mutants(text, &tokens(text, &vocabulary)))
        {
            let sampled = index % SAMPLED_ONE_IN == 0 && (index / SAMPLED_ONE_IN) % 2 == half;
            index += 1;
            if !sampled {
                continue;
            }
            checked += 1;
            failures.extend(drawn_failure(&compiler, &mutant));
        }
    }
    assert!(checked > 5_000 / 2, "{checked} mutants checked");
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

const RECURSIVE_READERS: [&str; 2] = [
    "input held: i32[..10] = []\nfn lowest(i):\n    let a = held[i]\n    if i >= len(held) then 127 else min(a, lowest(i + 1))\nClip c:\n    Note/(lowest(0))\n",
    "input held = [1, 2, 3]\nfn lowest(i):\n    let a = held[i]\n    if i >= 3 then 127 else min(a, lowest(i + 1))\nClip c:\n    Note/(lowest(0))\n",
];

#[test]
fn no_mutant_of_a_recursive_function_reading_a_cell_through_a_let_draws_an_internal_error() {
    let corpora = [(vocabulary(), RECURSIVE_READERS.map(str::to_string).to_vec())];
    let (failures, checked) =
        covered(|rows| covered_mutants_draw_no_internal_error(rows, &corpora));
    assert_eq!(failures, Vec::<String>::new());
    assert_eq!(checked, 550);
}

#[test]
#[ignore = "exhaustive"]
fn exhaustive_no_mutant_of_a_recursive_function_reading_a_cell_through_a_let_draws_an_internal_error()
 {
    let vocabulary = vocabulary();
    let compiler = Compiler::new(&vocabulary).expect("the example vocabulary compiles");
    let mut failures = Vec::new();
    for text in RECURSIVE_READERS {
        for mutant in mutants(text, &tokens(text, &vocabulary)) {
            failures.extend(drawn_failure(&compiler, &mutant));
        }
    }
    assert_eq!(failures, Vec::<String>::new());
}

#[test]
fn a_destination_sliced_by_a_cell_is_refused_as_an_array() {
    let targets = "Clip chorus:\n    Note as hook /72\nClip bridge:\n    Note as hook /67\n";
    support::assert_rows_in_every_order(
        &vocabulary(),
        &[
            (
                &[
                    "input section = 0\n",
                    "Clip verse:\n    Plays | gain=to.hook.velocity: [chorus, bridge][..section]\n",
                    targets,
                ],
                &[(Code::DestinationType, "[chorus, bridge][..section]")],
            ),
            (
                &[
                    "input section = 0\n",
                    "Clip verse:\n    Plays: [chorus, bridge][section..]\n",
                    targets,
                ],
                &[(Code::DestinationType, "[chorus, bridge][section..]")],
            ),
            (
                &[
                    "input section = 0\n",
                    "Clip verse:\n    Plays: [chorus, bridge][section]\n",
                    targets,
                ],
                &[],
            ),
        ],
    );
}
