mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use symphonyscript::syntax::lex;
use symphonyscript::{Code, Compiler, Severity, Source};
use symphonyscript_fixtures::{
    PENDING_EXAMPLES, SPEC_SYM_EXAMPLES, SpecExample, document_examples, dsp_project,
    music_project, pending_key, pending_lines, pending_spec_examples, spec_examples, spec_projects,
    vocabulary,
};

fn failure(
    compiler: &Compiler,
    example: &SpecExample,
    declared_elsewhere: &[&str],
) -> Option<&'static str> {
    let source = Source {
        path: "example.sym".into(),
        text: example.text.clone(),
    };
    if !lex(&source, &vocabulary()).diagnostics.is_empty() {
        return Some("does not lex");
    }
    let diagnostics = compiler.check(std::slice::from_ref(&source));
    if example.tag == "sym-error" {
        return support::errors(diagnostics)
            .is_empty()
            .then_some("is accepted");
    }
    if !support::parsed(&example.text).diagnostics.is_empty() {
        return Some("does not parse");
    }
    let checks = if declared_elsewhere.is_empty() {
        support::errors(diagnostics).is_empty()
    } else {
        let undeclared: BTreeSet<&str> = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == Code::UnknownName)
            .map(|diagnostic| {
                &example.text
                    [diagnostic.span.range.start as usize..diagnostic.span.range.end as usize]
            })
            .collect();
        undeclared == declared_elsewhere.iter().copied().collect()
    };
    (!checks).then_some("does not check")
}

fn spec_example_failure(compiler: &Compiler, example: &SpecExample) -> Option<&'static str> {
    let declared_elsewhere = SPEC_SYM_EXAMPLES
        .iter()
        .find(|(section, ordinal, _)| {
            example.tag == "sym" && *section == example.section && *ordinal == example.ordinal
        })
        .map_or(&[][..], |(_, _, names)| names);
    failure(compiler, example, declared_elsewhere)
}

fn pending_list_problems(
    document: &str,
    examples: Vec<SpecExample>,
    list: &Path,
    write: bool,
    failure: impl Fn(&SpecExample) -> Option<&'static str>,
) -> Vec<String> {
    let examples: Vec<(String, SpecExample)> = examples
        .into_iter()
        .filter(|example| example.tag == "sym" || example.tag == "sym-error")
        .map(|example| (pending_key(document, &example), example))
        .collect();
    if write {
        let failing: BTreeSet<&String> = examples
            .iter()
            .filter(|(_, example)| failure(example).is_some())
            .map(|(key, _)| key)
            .collect();
        let text: String = failing.iter().map(|key| format!("{key}\n")).collect();
        std::fs::write(list, text)
            .unwrap_or_else(|error| panic!("{} writes: {error}", list.display()));
        return vec![format!(
            "wrote {} with {} lines; run again without SYMPHONY_PENDING_EXAMPLES to check it",
            list.display(),
            failing.len()
        )];
    }
    let listed = pending_lines(list);
    let keys: BTreeSet<&String> = examples.iter().map(|(key, _)| key).collect();
    let mut problems: Vec<String> = examples
        .iter()
        .filter_map(
            |(key, example)| match (listed.contains(key), failure(example)) {
                (true, None) => Some(format!(
                    "{} holds the rule: remove the line `{key}` from {}",
                    example.name(),
                    list.display()
                )),
                (false, Some(failure)) => Some(format!(
                    "{} {failure}: add the line `{key}` to {}",
                    example.name(),
                    list.display()
                )),
                _ => None,
            },
        )
        .collect();
    problems.extend(
        listed
            .iter()
            .filter(|line| !keys.contains(line))
            .map(|line| {
                format!(
                    "no example is `{line}`: remove the line from {}",
                    list.display()
                )
            }),
    );
    problems
}

#[test]
fn every_sym_example_that_declares_its_names_and_every_fixture_project_passes_check() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let pending = pending_spec_examples();
    let projects = [music_project(), dsp_project()].into_iter().chain(
        spec_projects()
            .into_iter()
            .filter(|project| !pending.contains(&project.name)),
    );
    for project in projects {
        let sources: Vec<Source> = project
            .files
            .iter()
            .map(|file| Source {
                path: file.path.clone(),
                text: file.text.clone(),
            })
            .collect();
        let errors: Vec<_> = compiler
            .check(&sources)
            .into_iter()
            .filter(|diagnostic| diagnostic.severity() == Severity::Error)
            .collect();
        assert_eq!(errors, vec![], "{}", project.name);
    }
}

#[test]
fn every_sym_example_uses_undeclared_exactly_the_names_its_entry_lists() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let pending = pending_spec_examples();
    for example in spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym" && !pending.contains(&example.name()))
    {
        let listed: BTreeSet<&str> = SPEC_SYM_EXAMPLES
            .iter()
            .find(|(section, ordinal, _)| {
                *section == example.section && *ordinal == example.ordinal
            })
            .map(|(_, _, names)| names.iter().copied().collect())
            .unwrap_or_else(|| panic!("{} has an entry", example.name()));
        let source = Source {
            path: "example.sym".into(),
            text: example.text.clone(),
        };
        let undeclared: BTreeSet<&str> = compiler
            .check(&[source])
            .iter()
            .filter(|diagnostic| diagnostic.code == Code::UnknownName)
            .map(|diagnostic| {
                let range =
                    diagnostic.span.range.start as usize..diagnostic.span.range.end as usize;
                &example.text[range]
            })
            .collect();
        assert_eq!(undeclared, listed, "{}", example.name());
    }
}

#[test]
fn every_sym_error_example_is_rejected() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let pending = pending_spec_examples();
    let rejected: Vec<String> = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym-error")
        .map(|example| {
            if !pending.contains(&example.name()) {
                let source = Source {
                    path: "example.sym".into(),
                    text: example.text.clone(),
                };
                let errors = support::errors(compiler.check(&[source]));
                assert!(!errors.is_empty(), "{} is accepted", example.name());
            }
            example.name()
        })
        .collect();
    assert_eq!(
        rejected,
        [
            "spec-4.1-sym-error-1",
            "spec-6.3-sym-error-1",
            "spec-6.6-sym-error-1"
        ]
    );
}

#[test]
fn the_sym_error_example_of_4_1_is_rejected_for_the_reason_beside_it() {
    let text = "Clip melody:\n    Note/300000000000\n";
    assert_eq!(
        support::check_codes(text),
        vec![(symphonyscript::Code::DoesNotFit, 22, 34)]
    );
}

#[test]
fn the_pending_list_names_exactly_the_spec_examples_that_fail_the_rule() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let write = std::env::var("SYMPHONY_PENDING_EXAMPLES").is_ok_and(|mode| mode == "write");
    let problems = pending_list_problems(
        "SPEC.md",
        spec_examples(),
        Path::new(PENDING_EXAMPLES),
        write,
        |example| spec_example_failure(&compiler, example),
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

const SMALL_DOCUMENT: &str = "## 1. Small\n\n```sym\ninput x = 1\n```\n\n```sym-error\ninput y = 2\n```\n\n### 1.1 Smaller\n\n```text\nnot an example\n```\n\n```sym\nClip melody:\n    Note/300000000000\n```\n\n### 1.2 Unbuilt\n\n```sym\ninput = 1\n```\n";

fn small_document_problems(list: &Path, write: bool) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    pending_list_problems(
        "SMALL.md",
        document_examples(SMALL_DOCUMENT),
        list,
        write,
        |example| failure(&compiler, example, &[]),
    )
}

fn list_file(name: &str, text: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, text).expect("the list writes");
    path
}

#[test]
fn a_listed_example_that_holds_the_rule_fails_naming_its_line() {
    let list = list_file(
        "pending-listed-holds.txt",
        "# pending\n\nSMALL.md §1 1 sym\nSMALL.md §1 1 sym-error\nSMALL.md §1.1 1 sym\nSMALL.md §1.2 1 sym\n",
    );
    assert_eq!(
        small_document_problems(&list, false),
        [format!(
            "spec-1-sym-1 holds the rule: remove the line `SMALL.md §1 1 sym` from {}",
            list.display()
        )]
    );
}

#[test]
fn an_unlisted_example_that_fails_check_fails_naming_its_line() {
    let list = list_file(
        "pending-unlisted-fails.txt",
        "SMALL.md §1 1 sym-error\nSMALL.md §1.2 1 sym\n",
    );
    assert_eq!(
        small_document_problems(&list, false),
        [format!(
            "spec-1.1-sym-1 does not check: add the line `SMALL.md §1.1 1 sym` to {}",
            list.display()
        )]
    );
}

#[test]
fn an_unlisted_example_that_does_not_parse_fails_naming_its_line() {
    let list = list_file(
        "pending-unlisted-unparsed.txt",
        "SMALL.md §1 1 sym-error\nSMALL.md §1.1 1 sym\n",
    );
    assert_eq!(
        small_document_problems(&list, false),
        [format!(
            "spec-1.2-sym-1 does not parse: add the line `SMALL.md §1.2 1 sym` to {}",
            list.display()
        )]
    );
}

#[test]
fn an_unlisted_sym_error_example_that_is_accepted_fails_naming_its_line() {
    let list = list_file(
        "pending-unlisted-accepted.txt",
        "SMALL.md §1.1 1 sym\nSMALL.md §1.2 1 sym\n",
    );
    assert_eq!(
        small_document_problems(&list, false),
        [format!(
            "spec-1-sym-error-1 is accepted: add the line `SMALL.md §1 1 sym-error` to {}",
            list.display()
        )]
    );
}

#[test]
fn a_listed_line_that_names_no_example_fails_naming_it() {
    let list = list_file(
        "pending-no-example.txt",
        "SMALL.md §1 1 sym-error\nSMALL.md §1.1 1 sym\nSMALL.md §1.2 1 sym\nSMALL.md §1 2 sym\n",
    );
    assert_eq!(
        small_document_problems(&list, false),
        [format!(
            "no example is `SMALL.md §1 2 sym`: remove the line from {}",
            list.display()
        )]
    );
}

#[test]
fn a_list_that_names_exactly_the_failing_examples_passes() {
    let list = list_file(
        "pending-exact.txt",
        "SMALL.md §1.2 1 sym\nSMALL.md §1.1 1 sym\nSMALL.md §1 1 sym-error\n",
    );
    assert_eq!(small_document_problems(&list, false), Vec::<String>::new());
}

#[test]
fn the_write_mode_writes_the_failing_examples_sorted_and_fails() {
    let list = list_file("pending-write.txt", "SMALL.md §1 1 sym\n");
    assert_eq!(
        small_document_problems(&list, true),
        [format!(
            "wrote {} with 3 lines; run again without SYMPHONY_PENDING_EXAMPLES to check it",
            list.display()
        )]
    );
    assert_eq!(
        std::fs::read_to_string(&list).expect("the list reads"),
        "SMALL.md §1 1 sym-error\nSMALL.md §1.1 1 sym\nSMALL.md §1.2 1 sym\n"
    );
    assert_eq!(small_document_problems(&list, false), Vec::<String>::new());
}
