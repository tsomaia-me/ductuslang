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
        assert!(
            sections.contains(&code.section()),
            "{code} names §{}",
            code.section()
        );
    }
}

#[test]
fn every_lexer_and_parser_code_is_an_error() {
    for code in Code::ALL {
        assert_eq!(code.severity(), Severity::Error, "{code}");
        assert_eq!(code.to_string(), code.as_str());
    }
}
