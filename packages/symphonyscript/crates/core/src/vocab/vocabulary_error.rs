use alloc::string::String;
use core::fmt;

use super::count::Count;

#[derive(Clone, PartialEq, Debug)]
pub enum VocabularyError {
    NotAsciiIdentifier {
        name: String,
    },
    ReservedName {
        name: String,
    },
    BuiltinName {
        name: String,
    },
    VariantName {
        name: String,
    },
    DuplicateName {
        name: String,
    },
    DuplicateAttribute {
        owner: String,
        name: String,
    },
    DuplicateDefaultAttribute {
        owner: String,
    },
    DuplicateFlag {
        owner: String,
        flag: char,
    },
    InvalidFlag {
        owner: String,
        attribute: String,
        flag: char,
    },
    FlagOnNonBool {
        owner: String,
        attribute: String,
    },
    CountRange {
        owner: String,
        count: Count,
    },
    ForeignHandle {
        referrer: Option<String>,
    },
    EmptyEnum {
        name: String,
    },
    EnumTooLarge {
        name: String,
        variants: usize,
    },
    UnknownDefaultVariant {
        owner: String,
        attribute: String,
        variant: String,
    },
    InvalidScale {
        unit: String,
        scale: f64,
    },
    RepeatedUnit {
        unit: String,
    },
    UnknownDefaultUnit {
        owner: String,
        attribute: String,
        unit: String,
    },
    NumericUnitName {
        unit: String,
    },
    FractionalScale {
        owner: String,
        attribute: String,
        unit: String,
    },
}

impl fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VocabularyError::NotAsciiIdentifier { name } => {
                write!(f, "`{name}` is not an ASCII identifier")
            }
            VocabularyError::ReservedName { name } => {
                write!(f, "`{name}` is a keyword or reserved word")
            }
            VocabularyError::BuiltinName { name } => write!(f, "`{name}` is a built-in name"),
            VocabularyError::VariantName { name } => {
                write!(f, "`{name}` is a variant of one of the vocabulary's enums")
            }
            VocabularyError::DuplicateName { name } => {
                write!(f, "two declarations are named `{name}`")
            }
            VocabularyError::DuplicateAttribute { owner, name } => {
                write!(f, "`{owner}` declares two attributes named `{name}`")
            }
            VocabularyError::DuplicateDefaultAttribute { owner } => {
                write!(f, "`{owner}` has two default attributes")
            }
            VocabularyError::DuplicateFlag { owner, flag } => {
                write!(f, "two attributes of `{owner}` share the flag `{flag}`")
            }
            VocabularyError::InvalidFlag {
                owner,
                attribute,
                flag,
            } => write!(
                f,
                "`{owner}.{attribute}` has the flag `{flag}`, which is not a flag character"
            ),
            VocabularyError::FlagOnNonBool { owner, attribute } => {
                write!(f, "`{owner}.{attribute}` has a flag but is not a `bool`")
            }
            VocabularyError::CountRange { owner, count } => {
                write!(
                    f,
                    "a count of `{owner}` has a minimum above its maximum: {count:?}"
                )
            }
            VocabularyError::ForeignHandle {
                referrer: Some(name),
            } => {
                write!(f, "`{name}` refers to a declaration of another vocabulary")
            }
            VocabularyError::ForeignHandle { referrer: None } => {
                write!(f, "a declaration of another vocabulary is configured")
            }
            VocabularyError::EmptyEnum { name } => write!(f, "enum `{name}` has no variants"),
            VocabularyError::EnumTooLarge { name, variants } => {
                write!(f, "enum `{name}` has {variants} variants, more than 65536")
            }
            VocabularyError::UnknownDefaultVariant {
                owner,
                attribute,
                variant,
            } => write!(
                f,
                "the default `{variant}` of `{owner}.{attribute}` is not a variant of its enum"
            ),
            VocabularyError::InvalidScale { unit, scale } => {
                write!(
                    f,
                    "unit `{unit}` has the scale {scale}, which is not a positive finite number"
                )
            }
            VocabularyError::RepeatedUnit { unit } => {
                write!(f, "the unit name `{unit}` is declared twice")
            }
            VocabularyError::UnknownDefaultUnit {
                owner,
                attribute,
                unit,
            } => write!(
                f,
                "the default unit `{unit}` of `{owner}.{attribute}` is not in its family"
            ),
            VocabularyError::NumericUnitName { unit } => write!(
                f,
                "the unit name `{unit}` would read as part of a number literal after a digit"
            ),
            VocabularyError::FractionalScale {
                owner,
                attribute,
                unit,
            } => write!(
                f,
                "the integer attribute `{owner}.{attribute}` belongs to a family whose unit `{unit}` has a scale that is not an integer"
            ),
        }
    }
}
