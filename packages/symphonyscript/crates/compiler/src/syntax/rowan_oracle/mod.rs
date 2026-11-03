mod element_view;
mod lexer;
mod navigation;
mod node_view;
mod parsed;
mod parser;
mod symphony_language;
mod syntax_element;
mod syntax_node;
mod syntax_token;
mod token_view;
mod walk;

use symphonyscript_core::Vocabulary;

pub(crate) use element_view::ElementView;
pub(crate) use parsed::Parsed;
pub(crate) use walk::Walk;

use crate::source::Source;
use crate::syntax::Lexed;

pub(crate) fn lex(source: &Source, vocabulary: &Vocabulary) -> Lexed {
    lexer::Lexer::new(source, vocabulary).run()
}

pub(crate) fn parse(source: &Source, vocabulary: &Vocabulary) -> Parsed {
    let (green, diagnostics, detached) = parser::Parser::new(source, vocabulary).run();
    Parsed::new(green, diagnostics, detached)
}
