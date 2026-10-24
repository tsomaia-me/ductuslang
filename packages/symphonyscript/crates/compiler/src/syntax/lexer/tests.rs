use symphonyscript_core::Vocabulary;

use super::Lexer;
use crate::source::Source;

const DEPTH: usize = 1_500;

fn nested_bodies() -> String {
    let mut text = String::new();
    for level in 0..DEPTH {
        text.push_str(&" ".repeat(level));
        text.push_str("Note:\n");
    }
    text.push_str(&" ".repeat(DEPTH));
    text
}

#[test]
fn indentation_stacks_are_stored_once_per_line_that_opens_a_bracket() {
    let long_bracket = format!(
        "{}Note/(1 +\n{}1)\n",
        nested_bodies(),
        "1 +\n".repeat(60_000)
    );
    let many_groups = format!("{}Note{}\n", nested_bodies(), " (Note)".repeat(100_000));
    for (text, regions, suspended) in [(long_bracket, 1, 60_001), (many_groups, 100_000, 0)] {
        let source = Source {
            path: "deep.sym".into(),
            text,
        };
        let vocabulary = Vocabulary::builder("empty")
            .build()
            .expect("an empty vocabulary is valid");
        let mut lexer = Lexer::new(&source, &vocabulary);
        while lexer.advance() {}

        assert!(lexer.diagnostics.is_empty());
        assert_eq!(lexer.regions.len(), regions);
        assert_eq!(lexer.suspended.len(), suspended);
        let stored: usize = lexer.stacks.iter().map(Vec::len).sum();
        assert_eq!(stored, DEPTH + 1);
    }
}
