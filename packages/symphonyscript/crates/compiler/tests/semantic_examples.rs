mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use symphonyscript::{Code, Compiler, Severity, Source};
use symphonyscript_fixtures::{
    SPEC_SYM_EXAMPLES, SpecExample, document_examples, dsp_project, music_project, spec_examples,
    spec_projects, vocabulary,
};

const PENDING_EXAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/pending-examples.txt"
);

fn keyed(document: &str, examples: Vec<SpecExample>) -> Vec<(String, SpecExample)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    examples
        .into_iter()
        .filter(|example| example.tag == "sym" || example.tag == "sym-error")
        .map(|example| {
            let ordinal = counts.entry(example.section.clone()).or_default();
            *ordinal += 1;
            (
                format!("{document} §{} {ordinal} {}", example.section, example.tag),
                example,
            )
        })
        .collect()
}

fn listed(list: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(list)
        .unwrap_or_else(|error| panic!("{} reads: {error}", list.display()))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(String::from)
        .collect()
}

fn pending_spec_examples() -> BTreeSet<String> {
    let listed = listed(Path::new(PENDING_EXAMPLES));
    keyed("SPEC.md", spec_examples())
        .into_iter()
        .filter(|(key, _)| listed.contains(key))
        .map(|(_, example)| example.name())
        .collect()
}

fn holds(compiler: &Compiler, example: &SpecExample, declared_elsewhere: &[&str]) -> bool {
    let source = Source {
        path: "example.sym".into(),
        text: example.text.clone(),
    };
    let diagnostics = compiler.check(&[source]);
    if example.tag == "sym-error" {
        return !support::errors(diagnostics).is_empty();
    }
    if !support::parsed(&example.text).diagnostics.is_empty() {
        return false;
    }
    if declared_elsewhere.is_empty() {
        return support::errors(diagnostics).is_empty();
    }
    let undeclared: BTreeSet<&str> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == Code::UnknownName)
        .map(|diagnostic| {
            &example.text[diagnostic.span.range.start as usize..diagnostic.span.range.end as usize]
        })
        .collect();
    undeclared == declared_elsewhere.iter().copied().collect()
}

fn spec_example_holds(compiler: &Compiler, example: &SpecExample) -> bool {
    let declared_elsewhere = SPEC_SYM_EXAMPLES
        .iter()
        .find(|(section, ordinal, _)| {
            example.tag == "sym" && *section == example.section && *ordinal == example.ordinal
        })
        .map_or(&[][..], |(_, _, names)| names);
    holds(compiler, example, declared_elsewhere)
}

fn pending_list_problems(
    document: &str,
    examples: Vec<SpecExample>,
    list: &Path,
    write: bool,
    holds: impl Fn(&SpecExample) -> bool,
) -> Vec<String> {
    let examples = keyed(document, examples);
    if write {
        let failing: BTreeSet<&String> = examples
            .iter()
            .filter(|(_, example)| !holds(example))
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
    let listed = listed(list);
    let keys: BTreeSet<&String> = examples.iter().map(|(key, _)| key).collect();
    let mut problems: Vec<String> = examples
        .iter()
        .filter_map(
            |(key, example)| match (listed.contains(key), holds(example)) {
                (true, true) => Some(format!(
                    "{} holds the rule: remove the line `{key}` from {}",
                    example.name(),
                    list.display()
                )),
                (false, false) => Some(format!(
                    "{} fails the rule: add the line `{key}` to {}",
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
        |example| spec_example_holds(&compiler, example),
    );
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

const SMALL_DOCUMENT: &str = "## 1. Small\n\n```sym\ninput x = 1\n```\n\n```sym-error\ninput y = 2\n```\n\n### 1.1 Smaller\n\n```text\nnot an example\n```\n\n```sym\nClip melody:\n    Note/300000000000\n```\n";

fn small_document_problems(list: &Path, write: bool) -> Vec<String> {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    pending_list_problems(
        "SMALL.md",
        document_examples(SMALL_DOCUMENT),
        list,
        write,
        |example| holds(&compiler, example, &[]),
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
        "# pending\n\nSMALL.md §1 1 sym\nSMALL.md §1 2 sym-error\nSMALL.md §1.1 1 sym\n",
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
fn an_unlisted_example_that_fails_the_rule_fails_naming_its_line() {
    let list = list_file("pending-unlisted-fails.txt", "SMALL.md §1 2 sym-error\n");
    assert_eq!(
        small_document_problems(&list, false),
        [format!(
            "spec-1.1-sym-1 fails the rule: add the line `SMALL.md §1.1 1 sym` to {}",
            list.display()
        )]
    );
}

#[test]
fn a_listed_line_that_names_no_example_fails_naming_it() {
    let list = list_file(
        "pending-no-example.txt",
        "SMALL.md §1 2 sym-error\nSMALL.md §1.1 1 sym\nSMALL.md §1 3 sym\n",
    );
    assert_eq!(
        small_document_problems(&list, false),
        [format!(
            "no example is `SMALL.md §1 3 sym`: remove the line from {}",
            list.display()
        )]
    );
}

#[test]
fn a_list_that_names_exactly_the_failing_examples_passes() {
    let list = list_file(
        "pending-exact.txt",
        "SMALL.md §1.1 1 sym\nSMALL.md §1 2 sym-error\n",
    );
    assert_eq!(small_document_problems(&list, false), Vec::<String>::new());
}

#[test]
fn the_write_mode_writes_the_failing_examples_sorted_and_fails() {
    let list = list_file("pending-write.txt", "SMALL.md §1 1 sym\n");
    assert_eq!(
        small_document_problems(&list, true),
        [format!(
            "wrote {} with 2 lines; run again without SYMPHONY_PENDING_EXAMPLES to check it",
            list.display()
        )]
    );
    assert_eq!(
        std::fs::read_to_string(&list).expect("the list reads"),
        "SMALL.md §1 2 sym-error\nSMALL.md §1.1 1 sym\n"
    );
    assert_eq!(small_document_problems(&list, false), Vec::<String>::new());
}
