use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{bar_vocabulary, envelope_vocabulary, native_vocabulary, vocabulary};

pub const VOCABULARY_NAMES: [&str; 4] = ["main", "native", "envelope", "bar"];

pub fn named_vocabulary(name: &str) -> Result<Vocabulary, String> {
    match name {
        "main" => Ok(vocabulary()),
        "native" => Ok(native_vocabulary()),
        "envelope" => Ok(envelope_vocabulary()),
        "bar" => Ok(bar_vocabulary()),
        _ => Err(format!(
            "unknown vocabulary `{name}`; the vocabularies are {}",
            VOCABULARY_NAMES.join(", ")
        )),
    }
}
