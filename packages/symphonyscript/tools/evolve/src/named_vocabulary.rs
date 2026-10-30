use symphonyscript_core::Vocabulary;
use symphonyscript_fixtures::{
    bar_vocabulary, bounded_vocabulary, envelope_vocabulary, native_vocabulary,
    signature_vocabulary, to_array_vocabulary, vocabulary,
};

pub const VOCABULARY_NAMES: [&str; 4] = ["main", "native", "envelope", "bar"];
pub const PROBE_VOCABULARY_NAMES: [&str; 3] = ["bounded", "signature", "to"];
pub const PROBE_TAGS: [(&str, &str); 7] = [
    ("example", "main"),
    ("native", "native"),
    ("bounded", "bounded"),
    ("envelope", "envelope"),
    ("bar", "bar"),
    ("signature", "signature"),
    ("to", "to"),
];

pub fn tagged_vocabulary(tag: &str) -> Option<&'static str> {
    PROBE_TAGS
        .iter()
        .find(|(named, _)| *named == tag)
        .map(|(_, vocabulary)| *vocabulary)
}

pub fn named_vocabulary(name: &str) -> Result<Vocabulary, String> {
    match name {
        "main" => Ok(vocabulary()),
        "native" => Ok(native_vocabulary()),
        "envelope" => Ok(envelope_vocabulary()),
        "bar" => Ok(bar_vocabulary()),
        "bounded" => Ok(bounded_vocabulary()),
        "signature" => Ok(signature_vocabulary()),
        "to" => Ok(to_array_vocabulary()),
        _ => Err(format!(
            "unknown vocabulary `{name}`; the vocabularies are {}",
            [VOCABULARY_NAMES.as_slice(), &PROBE_VOCABULARY_NAMES]
                .concat()
                .join(", ")
        )),
    }
}
