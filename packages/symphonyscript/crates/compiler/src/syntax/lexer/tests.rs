use symphonyscript_core::Vocabulary;

use super::Lexer;
use crate::source::Source;

#[test]
fn a_bracket_region_stores_one_indentation_stack_for_all_its_lines() {
    let depth = 1_500;
    let lines = 60_000;
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(&" ".repeat(level));
        text.push_str("Clip c:\n");
    }
    text.push_str(&" ".repeat(depth));
    text.push_str("Note/(1 +\n");
    for _ in 0..lines {
        text.push_str("1 +\n");
    }
    text.push_str("1)\n");
    let source = Source {
        path: "deep.sym".into(),
        text,
    };
    let vocabulary = Vocabulary::builder("empty")
        .build()
        .expect("an empty vocabulary is valid");
    let mut lexer = Lexer::new(&source, &vocabulary);
    while lexer.advance() {}

    assert_eq!(lexer.suspended.len(), lines + 1);
    assert!(lexer.suspended.iter().all(|line| line.region == 0));
    assert_eq!(lexer.regions.len(), 1);
    let stored: usize = lexer.regions.iter().map(|region| region.stack.len()).sum();
    assert_eq!(stored, depth + 1);
}
