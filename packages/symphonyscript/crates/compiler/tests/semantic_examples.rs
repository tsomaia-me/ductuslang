mod support;

use symphonyscript::{Compiler, Severity, Source};
use symphonyscript_fixtures::{dsp_project, music_project, spec_projects, vocabulary};

#[test]
fn every_sym_example_that_declares_its_names_and_every_fixture_project_passes_check() {
    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let projects = [music_project(), dsp_project()]
        .into_iter()
        .chain(spec_projects());
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
    use std::collections::BTreeSet;
    use symphonyscript::Code;
    use symphonyscript_fixtures::{SPEC_SYM_EXAMPLES, spec_examples};

    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    for example in spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym")
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
    use symphonyscript_fixtures::spec_examples;

    let compiler = Compiler::new(&vocabulary()).expect("the example vocabulary compiles");
    let rejected: Vec<String> = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym-error")
        .map(|example| {
            let source = Source {
                path: "example.sym".into(),
                text: example.text.clone(),
            };
            let errors = support::errors(compiler.check(&[source]));
            assert!(!errors.is_empty(), "{} is accepted", example.name());
            example.name()
        })
        .collect();
    assert_eq!(rejected, ["spec-4.1-sym-error-1"]);
}

#[test]
fn the_sym_error_example_of_4_1_is_rejected_for_the_reason_beside_it() {
    let text = "Clip melody:\n    Note/300000000000\n";
    assert_eq!(
        support::check_codes(text),
        vec![(symphonyscript::Code::DoesNotFit, 22, 34)]
    );
}
