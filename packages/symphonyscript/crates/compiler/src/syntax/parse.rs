use symphonyscript_core::Vocabulary;

use super::parsed::Parsed;
use super::parser::Parser;
use crate::source::Source;

pub fn parse(source: &Source, vocabulary: &Vocabulary) -> Parsed {
    let (green, diagnostics, detached) = Parser::new(source, vocabulary).run();
    Parsed::new(green, diagnostics, detached)
}
