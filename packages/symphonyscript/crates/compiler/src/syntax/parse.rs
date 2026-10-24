use symphonyscript_core::Vocabulary;

use super::lexer::Lexer;
use super::parsed::Parsed;
use super::parser::Parser;
use crate::source::Source;

pub fn parse(source: &Source, vocabulary: &Vocabulary) -> Parsed {
    let lexed = Lexer::new(source, vocabulary).run();
    let (green, parser_diagnostics) = Parser::new(source, vocabulary, lexed.tokens).run();
    let mut diagnostics = lexed.diagnostics;
    diagnostics.extend(parser_diagnostics);
    diagnostics.sort_by_key(|diagnostic| diagnostic.span.range.start);
    Parsed { green, diagnostics }
}
