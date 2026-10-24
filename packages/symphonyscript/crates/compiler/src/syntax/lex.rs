use symphonyscript_core::Vocabulary;

use super::lexed::Lexed;
use super::lexer::Lexer;
use crate::source::Source;

pub fn lex(source: &Source, vocabulary: &Vocabulary) -> Lexed {
    Lexer::new(source, vocabulary).run()
}
