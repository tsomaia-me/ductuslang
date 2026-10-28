use symphonyscript_evolve::{
    Checker, DEFAULT_BUDGET, Program, ProgramSource, VOCABULARY_NAMES, named_vocabulary,
};

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
