use symphonyscript_core::Vocabulary;

use crate::vocabulary_documented::vocabulary_documented;

pub fn vocabulary() -> Vocabulary {
    vocabulary_documented(|text| text.to_string())
}
