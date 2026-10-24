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
    TopLevelSyntax,
    UnexpectedIndent,
    TypeSyntax,
    ExpressionSyntax,
    ChainedComparison,
    UnparenthesizedIf,
    MatchSyntax,
    MatchPosition,
    MixedArguments,
    ConversionSyntax,
    InputSyntax,
    LetSyntax,
    FnSyntax,
    PatternSyntax,
    PlacementSyntax,
    TopLevelWire,
    FlagSyntax,
    DefaultArgSyntax,
    AttributeSyntax,
    InlinePlacement,
    BundleSyntax,
    ForSyntax,
    WireSyntax,
    MisplacedWire,
    WhenSyntax,
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
        Code::TopLevelSyntax,
        Code::UnexpectedIndent,
        Code::TypeSyntax,
        Code::ExpressionSyntax,
        Code::ChainedComparison,
        Code::UnparenthesizedIf,
        Code::MatchSyntax,
        Code::MatchPosition,
        Code::MixedArguments,
        Code::ConversionSyntax,
        Code::InputSyntax,
        Code::LetSyntax,
        Code::FnSyntax,
        Code::PatternSyntax,
        Code::PlacementSyntax,
        Code::TopLevelWire,
        Code::FlagSyntax,
        Code::DefaultArgSyntax,
        Code::AttributeSyntax,
        Code::InlinePlacement,
        Code::BundleSyntax,
        Code::ForSyntax,
        Code::WireSyntax,
        Code::MisplacedWire,
        Code::WhenSyntax,
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
            Code::TopLevelSyntax => "top-level-syntax",
            Code::UnexpectedIndent => "unexpected-indent",
            Code::TypeSyntax => "type-syntax",
            Code::ExpressionSyntax => "expression-syntax",
            Code::ChainedComparison => "chained-comparison",
            Code::UnparenthesizedIf => "unparenthesized-if",
            Code::MatchSyntax => "match-syntax",
            Code::MatchPosition => "match-position",
            Code::MixedArguments => "mixed-arguments",
            Code::ConversionSyntax => "conversion-syntax",
            Code::InputSyntax => "input-syntax",
            Code::LetSyntax => "let-syntax",
            Code::FnSyntax => "fn-syntax",
            Code::PatternSyntax => "pattern-syntax",
            Code::PlacementSyntax => "placement-syntax",
            Code::TopLevelWire => "top-level-wire",
            Code::FlagSyntax => "flag-syntax",
            Code::DefaultArgSyntax => "default-arg-syntax",
            Code::AttributeSyntax => "attribute-syntax",
            Code::InlinePlacement => "inline-placement",
            Code::BundleSyntax => "bundle-syntax",
            Code::ForSyntax => "for-syntax",
            Code::WireSyntax => "wire-syntax",
            Code::MisplacedWire => "misplaced-wire",
            Code::WhenSyntax => "when-syntax",
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
            Code::TopLevelSyntax => "2.1",
            Code::UnexpectedIndent => "2.2",
            Code::TypeSyntax => "3.2",
            Code::ExpressionSyntax | Code::ChainedComparison | Code::UnparenthesizedIf => "5.1",
            Code::MatchSyntax | Code::MatchPosition => "5.4",
            Code::MixedArguments => "5.7",
            Code::ConversionSyntax => "5.8",
            Code::InputSyntax => "6.1",
            Code::LetSyntax => "6.2",
            Code::FnSyntax => "6.3",
            Code::PatternSyntax => "6.4",
            Code::PlacementSyntax | Code::TopLevelWire => "7.1",
            Code::FlagSyntax => "7.4",
            Code::DefaultArgSyntax => "7.5",
            Code::AttributeSyntax => "7.6",
            Code::InlinePlacement => "7.7",
            Code::BundleSyntax => "7.8",
            Code::ForSyntax => "7.9",
            Code::WireSyntax | Code::MisplacedWire => "8.1",
            Code::WhenSyntax => "9.2",
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
