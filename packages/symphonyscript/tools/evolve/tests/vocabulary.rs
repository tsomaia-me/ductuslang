use symphonyscript_evolve::{
    Checker, DEFAULT_BUDGET, PROBE_VOCABULARY_NAMES, Program, ProgramSource, VOCABULARY_NAMES,
    named_vocabulary,
};

#[test]
fn a_probe_vocabulary_is_the_one_its_probe_tag_names_and_compiles() {
    let named: Vec<String> = PROBE_VOCABULARY_NAMES
        .into_iter()
        .map(|name| {
            let vocabulary = named_vocabulary(name).expect("a probe vocabulary");
            Checker::with_vocabulary(DEFAULT_BUDGET, vocabulary.clone())
                .expect("the vocabulary compiles");
            vocabulary.name().to_string()
        })
        .collect();
    assert_eq!(named, ["bounded", "signatures", "to-arrays"]);
    let unknown = named_vocabulary("example").expect_err("example is a tag, not a vocabulary");
    assert!(unknown.ends_with("main, native, envelope, bar, bounded, signature, to"));
}

const ENTRY_WARNING: &str = "no-entry-instance";

#[test]
fn a_named_vocabulary_accepts_exactly_its_own_names() {
    let programs = [
        ("native", "let a = checksum(\"GET\")\nClip c:\n    Note/a\n"),
        ("envelope", "Env e | attack=3ms\n"),
        ("bar", "Bar b | bar_len=2bars\n"),
    ];
    for name in VOCABULARY_NAMES {
        let vocabulary = named_vocabulary(name).expect("a named vocabulary");
        let checker =
            Checker::with_vocabulary(DEFAULT_BUDGET, vocabulary).expect("the vocabulary compiles");
        for (owner, text) in programs {
            let program = Program {
                id: serde_json::json!(owner),
                sources: vec![ProgramSource {
                    path: "main.sym".to_string(),
                    text: text.to_string(),
                }],
            };
            let errors: Vec<String> = checker
                .verdict(&program)
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.code)
                .filter(|code| code != ENTRY_WARNING)
                .collect();
            assert_eq!(
                errors.is_empty(),
                owner == name,
                "the {owner} program under the {name} vocabulary draws {errors:?}"
            );
        }
    }
    assert!(named_vocabulary("music").is_err());
}
