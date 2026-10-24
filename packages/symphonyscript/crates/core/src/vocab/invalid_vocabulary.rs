use alloc::vec::Vec;
use core::fmt;

use super::vocabulary_error::VocabularyError;

#[derive(Clone, PartialEq, Debug)]
pub struct InvalidVocabulary {
    pub errors: Vec<VocabularyError>,
}

impl fmt::Display for InvalidVocabulary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the vocabulary is invalid")?;
        for error in &self.errors {
            write!(f, "\n{error}")?;
        }
        Ok(())
    }
}

impl core::error::Error for InvalidVocabulary {}
