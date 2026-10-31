use symphonyscript::{Code, Severity};
use symphonyscript_fixtures::SPEC;

#[test]
fn every_code_is_kebab_case_and_unique() {
    let mut names: Vec<&str> = Code::ALL.iter().map(|code| code.as_str()).collect();
    for name in &names {
        assert!(
            name.split('-')
                .all(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_lowercase())),
            "{name}"
        );
    }
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), count);
}

#[test]
fn every_code_names_a_section_of_the_spec() {
    let sections: Vec<&str> = SPEC
        .lines()
        .filter_map(|line| {
            line.strip_prefix("## ")
                .or_else(|| line.strip_prefix("### "))
        })
        .filter_map(|heading| heading.split_whitespace().next())
        .map(|number| number.trim_end_matches('.'))
        .collect();
    for code in Code::ALL {
        match code.section() {
            Some(section) => assert!(sections.contains(&section), "{code} names §{section}"),
            None => assert!(
                matches!(code, Code::InternalError | Code::UnbuiltConstruct),
                "{code} names no section"
            ),
        }
    }
}

#[test]
fn the_warnings_are_the_four_the_spec_names_and_every_other_code_is_an_error() {
    let warnings = [
        "integer-cell",
        "unreachable-instance",
        "no-entry-instance",
        "limit-warning",
    ];
    for code in Code::ALL {
        let expected = if warnings.contains(&code.as_str()) {
            Severity::Warning
        } else {
            Severity::Error
        };
        assert_eq!(code.severity(), expected, "{code}");
        assert_eq!(code.to_string(), code.as_str());
    }
}

#[test]
fn every_code_carries_a_one_line_help() {
    for code in Code::ALL {
        let help = code.help().unwrap_or_default();
        assert!(
            !help.trim().is_empty() && !help.contains('\n'),
            "{code}: {help:?}"
        );
    }
}
