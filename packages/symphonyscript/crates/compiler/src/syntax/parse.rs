use symphonyscript_core::Vocabulary;

use super::lexer::Lexer;
use super::parsed::Parsed;
use super::parser::Parser;
use crate::source::Source;

pub fn parse(source: &Source, vocabulary: &Vocabulary) -> Parsed {
    let (lexed, layout) = Lexer::new(source, vocabulary).run_with_layout();
    let (green, diagnostics) = Parser::new(source, vocabulary, lexed, layout).run();
    Parsed::new(green, diagnostics)
}
