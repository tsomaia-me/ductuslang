use symphonyscript_core::Vocabulary;

use super::parsed::Parsed;
use super::parser::Parser;
use crate::source::Source;

pub fn parse(source: &Source, vocabulary: &Vocabulary) -> Parsed {
    Parser::new(source, vocabulary).run()
}
