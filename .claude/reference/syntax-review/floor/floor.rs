use symphonyscript_core::Vocabulary;

use super::lexer::Lexer;
use super::parsed::Parsed;
use crate::semantics::ir::{FlatIr, build_flat_file};
use crate::source::Source;

pub fn lex_columns(source: &Source, vocabulary: &Vocabulary) -> (usize, usize, usize) {
    let mut lexer = Lexer::new(source, vocabulary);
    while lexer.advance() {}
    let (columns, diagnostics) = lexer.finish();
    (columns.len(), columns.sig.len(), diagnostics.len())
}

pub fn counts(parsed: &Parsed) -> (usize, usize, usize) {
    (
        parsed.tokens().len(),
        parsed.tokens().sig.len(),
        parsed.tree().len(),
    )
}

pub struct Ir(FlatIr);

impl Ir {
    pub fn new() -> Self {
        Ir(FlatIr::default())
    }

    pub fn add(&mut self, path: &str, parsed: &Parsed, vocabulary: &Vocabulary) -> bool {
        build_flat_file(&mut self.0, path, parsed, vocabulary).is_ok()
    }

    pub fn nodes(&self) -> usize {
        self.0.op.len()
    }
}
