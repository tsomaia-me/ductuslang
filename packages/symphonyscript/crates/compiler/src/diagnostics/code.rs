use std::fmt;

use super::severity::Severity;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Code {
    UnexpectedCharacter,
    TabIndentation,
    DedentMismatch,
    ClauseColumn,
    Semicolon,
    InvalidIdentifier,
    ReservedWord,
    ReservedToken,
    MisplacedUnderscore,
    FloatDigits,
    FloatTypeSuffix,
    UnknownSuffix,
    PrefixedSuffix,
    IntegerTooLarge,
    UnterminatedString,
    InvalidEscape,
}

impl Code {
    pub const ALL: &'static [Code] = &[
        Code::UnexpectedCharacter,
        Code::TabIndentation,
        Code::DedentMismatch,
        Code::ClauseColumn,
        Code::Semicolon,
        Code::InvalidIdentifier,
        Code::ReservedWord,
        Code::ReservedToken,
        Code::MisplacedUnderscore,
        Code::FloatDigits,
        Code::FloatTypeSuffix,
        Code::UnknownSuffix,
        Code::PrefixedSuffix,
        Code::IntegerTooLarge,
        Code::UnterminatedString,
        Code::InvalidEscape,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Code::UnexpectedCharacter => "unexpected-character",
            Code::TabIndentation => "tab-indentation",
            Code::DedentMismatch => "dedent-mismatch",
            Code::ClauseColumn => "clause-column",
            Code::Semicolon => "semicolon",
            Code::InvalidIdentifier => "invalid-identifier",
            Code::ReservedWord => "reserved-word",
            Code::ReservedToken => "reserved-token",
            Code::MisplacedUnderscore => "misplaced-underscore",
            Code::FloatDigits => "float-digits",
            Code::FloatTypeSuffix => "float-type-suffix",
            Code::UnknownSuffix => "unknown-suffix",
            Code::PrefixedSuffix => "prefixed-suffix",
            Code::IntegerTooLarge => "integer-too-large",
            Code::UnterminatedString => "unterminated-string",
            Code::InvalidEscape => "invalid-escape",
        }
    }

    pub fn section(self) -> &'static str {
        match self {
            Code::UnexpectedCharacter => "2",
            Code::TabIndentation | Code::DedentMismatch | Code::ClauseColumn | Code::Semicolon => {
                "2.2"
            }
            Code::InvalidIdentifier => "2.4",
            Code::ReservedWord | Code::ReservedToken => "2.5",
            Code::MisplacedUnderscore
            | Code::FloatDigits
            | Code::FloatTypeSuffix
            | Code::UnknownSuffix
            | Code::PrefixedSuffix
            | Code::IntegerTooLarge
            | Code::UnterminatedString
            | Code::InvalidEscape => "2.6",
        }
    }

    pub fn severity(self) -> Severity {
        Severity::Error
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
