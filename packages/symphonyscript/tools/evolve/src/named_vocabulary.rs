#[path = "../../../crates/compiler/tests/support/mod.rs"]
mod support;

use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{
    bar_vocabulary, bounded_vocabulary, envelope_vocabulary, native_vocabulary, vocabulary,
};

pub const VOCABULARY_NAMES: [&str; 4] = ["main", "native", "envelope", "bar"];
pub const PROBE_VOCABULARY_NAMES: [&str; 3] = ["bounded", "signature", "to"];

pub fn named_vocabulary(name: &str) -> Result<Vocabulary, String> {
    match name {
        "main" => Ok(vocabulary()),
        "native" => Ok(native_vocabulary()),
        "envelope" => Ok(envelope_vocabulary()),
        "bar" => Ok(bar_vocabulary()),
        "bounded" => Ok(bounded_vocabulary()),
        "signature" => Ok(support::signature_vocabulary()),
        "to" => Ok(support::to_array_vocabulary()),
        _ => Err(format!(
            "unknown vocabulary `{name}`; the vocabularies are {}",
            [VOCABULARY_NAMES.as_slice(), &PROBE_VOCABULARY_NAMES]
                .concat()
                .join(", ")
        )),
    }
}
