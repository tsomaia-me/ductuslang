use symphonyscript::{Code, Compiler, Diagnostic, Severity, Source};
use symphonyscript_core::LimitOverrides;
use symphonyscript_fixtures::{
    bar_vocabulary, bounded_vocabulary, native_vocabulary, prelude_vocabulary, vocabulary,
};

struct Example {
    error: bool,
    setup: String,
    text: String,
}

fn examples(lesson: &str) -> Vec<Example> {
    let mut found = Vec::new();
    let mut open: Option<(bool, String, String)> = None;
    for line in lesson.lines() {
        if let Some((error, setup, text)) = open.as_mut() {
            if line == "```" {
                found.push(Example {
                    error: *error,
                    setup: setup.clone(),
                    text: text.clone(),
                });
                open = None;
            } else {
                text.push_str(line);
                text.push('\n');
            }
            continue;
        }
        let Some(info) = line.strip_prefix("```") else {
            continue;
        };
        let mut words = info.split_whitespace();
        let error = match words.next() {
            Some("sym") => false,
            Some("sym-error") => true,
            Some(_) | None => continue,
        };
        let setup = words.next().unwrap_or("").to_string();
        open = Some((error, setup, String::new()));
    }
    assert!(open.is_none(), "a lesson's example is unclosed");
    found
}

fn diagnostics(setup: &str, text: &str, limits: LimitOverrides) -> Vec<Diagnostic> {
    let source = Source {
        path: "lesson.sym".into(),
        text: text.into(),
    };
    let vocabulary = match setup {
        "at-prelude-path" => {
            return Compiler::new(&prelude_vocabulary(""))
                .expect("the prelude vocabulary compiles")
                .check(&[Source {
                    path: "prelude.sym".into(),
                    text: text.into(),
                }]);
        }
        "prelude" => {
            return match Compiler::new(&prelude_vocabulary(text)) {
                Ok(_) => Vec::new(),
                Err(failed) => failed.diagnostics,
            };
        }
        "" => vocabulary(),
        "natives" => native_vocabulary(),
        "bars" => bar_vocabulary(),
        "bounded" => bounded_vocabulary(),
        other => panic!("no lesson setup {other}"),
    };
    Compiler::with_limits(&vocabulary, limits)
        .expect("the vocabulary compiles")
        .check(&[source])
}

#[test]
fn every_code_has_a_lesson_that_names_its_code_and_its_section() {
    for code in Code::ALL {
        let lesson = code.lesson();
        assert!(
            lesson.starts_with(&format!("# {}\n\n", code.as_str())),
            "{code}"
        );
        if let Some(section) = code.section() {
            assert!(lesson.contains(&format!("(SPEC §{section})")), "{code}");
        }
        let examples = examples(lesson);
        if matches!(code, Code::InternalError | Code::UnbuiltConstruct) {
            assert!(examples.is_empty(), "{code}");
            continue;
        }
        assert_eq!(
            examples.iter().filter(|example| example.error).count(),
            1,
            "{code}"
        );
        assert!(examples.iter().any(|example| !example.error), "{code}");
    }
}

#[test]
fn every_lesson_draws_its_code_in_its_sym_error_example_and_passes_check_in_its_fix() {
    lessons_hold(
        Code::ALL
            .iter()
            .filter(|code| !LIMIT_LESSONS.contains(code)),
        LimitOverrides::default(),
    );
}

const LIMIT_LESSONS: [Code; 2] = [Code::RecursionLimit, Code::ExpansionLimit];

#[test]
fn the_limit_lessons_draw_their_codes_at_lowered_limits_in_their_error_examples_and_pass_in_their_fixes()
 {
    lessons_hold(
        LIMIT_LESSONS.iter(),
        LimitOverrides {
            recursive_calls: Some(1_000),
            expansion_steps: Some(1_000),
            expanded_items: Some(1_000),
            ..LimitOverrides::default()
        },
    );
}

#[test]
#[ignore = "stress"]
fn stress_the_limit_lessons_draw_their_codes_in_their_error_examples_and_pass_in_their_fixes() {
    lessons_hold(LIMIT_LESSONS.iter(), LimitOverrides::default());
}

fn lessons_hold<'a>(codes: impl Iterator<Item = &'a Code>, limits: LimitOverrides) {
    let mut failures = Vec::new();
    for code in codes {
        for example in examples(code.lesson()) {
            let found = diagnostics(&example.setup, &example.text, limits);
            let codes: Vec<Code> = found.iter().map(|diagnostic| diagnostic.code).collect();
            let errors = found
                .iter()
                .filter(|diagnostic| diagnostic.severity() == Severity::Error)
                .count();
            let holds = if example.error {
                codes.contains(code)
            } else {
                errors == 0 && !codes.contains(code)
            };
            if !holds {
                failures.push(format!("{code}: {:?} draws {codes:?}", example.text));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
