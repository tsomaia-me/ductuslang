mod support;

use symphonyscript_fixtures::spec_examples;

#[test]
fn every_sym_example_parses() {
    let examples: Vec<_> = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym")
        .collect();
    assert!(!examples.is_empty());
    for example in examples {
        let parsed = support::parsed(&example.text);
        assert_eq!(
            parsed.diagnostics,
            vec![],
            "{} (SPEC.md line {})",
            example.name(),
            example.line
        );
        assert_eq!(
            parsed.root().to_string(),
            example.text,
            "{}",
            example.name()
        );
    }
}

#[test]
fn every_sym_error_example_is_extracted() {
    let rejected: Vec<String> = spec_examples()
        .into_iter()
        .filter(|example| example.tag == "sym-error")
        .map(|example| {
            assert!(!example.text.is_empty(), "{}", example.name());
            assert_eq!(
                support::parsed(&example.text).root().to_string(),
                example.text
            );
            example.name()
        })
        .collect();
    assert_eq!(rejected, ["spec-4.1-sym-error-1"]);
}
