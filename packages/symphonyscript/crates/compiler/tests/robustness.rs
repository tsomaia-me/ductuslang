use std::panic::{AssertUnwindSafe, catch_unwind};

use symphonyscript::syntax::lex;
use symphonyscript::{Code, Compiler, Source};
use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{
    bar_vocabulary, dsp_project, envelope_vocabulary, music_project, spec_examples, vocabulary,
};

const INSERTED: &[&str] = &[
    "(", ")", "[", "]", ":", ",", "+", "-", "/", "x", "1", "when ", "\n",
];
const SAMPLED_ONE_IN: usize = 3;

const ENVELOPE_PROGRAMS: &[&str] = &[
    "Env e | attack=(1s + 3)\n",
    "Env e | attack=((1s + 3) / 2)\n",
    "Env e | attack=(1s + 3ms)\n",
    "Env e | attack=3i64\n",
    "Env e | attack=i64(3)\n",
    "Env e | attack=(1s + 3i32)\n",
    "Env e | release=3i32\n",
    "Env e | attack=(3i32 * 2.5)\n",
    "let p = 3\nEnv e | attack=(p + 1s) pitch=p\n",
    "let p = 3i32\nlet c = p > 1\nEnv e | attack=p pitch=p\nEnv f | attack=(if c then 1s else 2s)\n",
    "input d = 5ms\nlet e = d * 2\nEnv a | attack=e release=(d + 1)\n",
];

const BAR_PROGRAMS: &[&str] = &[
    "Bar b | len=(1bars + 1)\n",
    "let d = 2bars + 1\nBar b | len=d beat_len=d\n",
    "let k = (2bars + 1) div 1bars\nBar b | len=k\n",
    "let p = 3i64\nBar b | len=p beat_len=p plain=p\n",
    "fn g(): 1bars + 1\nlet x = g()\nBar b | len=(g()) beat_len=x\n",
    "fn r(n): if n < 1 then 2bars + 1 else r(n - 1)\nBar b | len=(r(2)) beat_len=(r(0) + 1)\n",
    "input n = 2\nBar b | len=(n + 1bars) beat_len=n\n",
];

fn corpora() -> Vec<(Vocabulary, Vec<String>)> {
    let mut texts: Vec<String> = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym")
        .map(|example| example.text)
        .collect();
    for project in [music_project(), dsp_project()] {
        texts.extend(project.files.into_iter().map(|file| file.text));
    }
    let owned = |programs: &[&str]| programs.iter().map(|text| text.to_string()).collect();
    vec![
        (vocabulary(), texts),
        (envelope_vocabulary(), owned(ENVELOPE_PROGRAMS)),
        (bar_vocabulary(), owned(BAR_PROGRAMS)),
    ]
}

fn mutants(text: &str, vocabulary: &Vocabulary) -> Vec<String> {
    let source = Source {
        path: "mutant.sym".into(),
        text: text.into(),
    };
    let ranges: Vec<(usize, usize)> = lex(&source, vocabulary)
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
    let mut checked = 0;
    let mut index = 0;
    let mut failures = Vec::new();
    for (vocabulary, texts) in corpora() {
        let compiler = Compiler::new(&vocabulary).expect("the vocabulary compiles");
        for mutant in texts.iter().flat_map(|text| mutants(text, &vocabulary)) {
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
