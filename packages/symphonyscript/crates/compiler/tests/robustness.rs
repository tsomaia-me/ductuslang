use std::panic::{AssertUnwindSafe, catch_unwind};

use symphonyscript::syntax::lex;
use symphonyscript::{Code, Compiler, Source};
use symphonyscript_fixtures::{dsp_project, music_project, spec_examples, vocabulary};

const INSERTED: &[&str] = &[
    "(", ")", "[", "]", ":", ",", "+", "-", "/", "x", "1", "when ", "\n",
];
const SAMPLED_ONE_IN: usize = 3;

fn originals() -> Vec<String> {
    let mut texts: Vec<String> = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym")
        .map(|example| example.text)
        .collect();
    for project in [music_project(), dsp_project()] {
        texts.extend(project.files.into_iter().map(|file| file.text));
    }
    texts
}

fn mutants(text: &str) -> Vec<String> {
    let source = Source {
        path: "mutant.sym".into(),
        text: text.into(),
    };
    let ranges: Vec<(usize, usize)> = lex(&source, &vocabulary())
        .tokens
        .iter()
        .filter_map(|token| {
            let start = usize::try_from(token.range.start).ok()?;
            let end = usize::try_from(token.range.end).ok()?;
            Some((start, end))
        })
        .collect();
    let mut found = Vec::new();
    for &(start, end) in &ranges {
        if start < end {
            let mut deleted = text.to_string();
            deleted.replace_range(start..end, "");
            found.push(deleted);
        }
        for inserted in INSERTED {
            let mut grown = text.to_string();
            grown.insert_str(start, inserted);
            found.push(grown);
        }
    }
    found
}

#[test]
fn no_mutant_of_an_example_or_fixture_draws_an_internal_error_or_panics() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let mut checked = 0;
    let mut failures = Vec::new();
    for (index, mutant) in originals()
        .iter()
        .flat_map(|text| mutants(text))
        .enumerate()
    {
        if index % SAMPLED_ONE_IN != 0 {
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
