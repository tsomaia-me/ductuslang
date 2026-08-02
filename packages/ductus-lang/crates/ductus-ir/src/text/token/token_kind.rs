//! The `TokenKind` enum: the closed `.duir` token inventory.

/// One token's kind, carrying its payload where it has one (§15.4.6).
///
/// The inventory is CLOSED at increment 3 (R19): it is the complete survey of
/// IR_GRAMMAR §2's lexemes and §3/§4's punctuation. A later construct increment
/// that needs a token absent here files a FINDING against the survey before the
/// variant is added — no silent growth.
///
/// Payloads are OWNED (R18). A `Vec<Token>` outlives the source slice it was
/// lexed from, so nothing downstream carries a lifetime; the cost is one
/// allocation per word-shaped token on a cold path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// A word-shaped run: letters, digits and `_`, not starting with a digit.
    ///
    /// EVERY word lexes as `Ident` — the lexer knows no keywords (R46). `module`,
    /// `init`, `true`, `nan`, `bb0` and a cell literally named `depth` all
    /// arrive here, and the parser classifies by position. Hard keywords would
    /// reject legal programs: a Ductus cell may be named `depth`.
    Ident(String),
    /// An integer lexeme as RAW TEXT, leading `-` included (033-247).
    ///
    /// Unconverted on purpose. The grammar admits `u128`/`i128` literals that no
    /// `i64` holds, so a lexer-side parse would either lose values or pick a
    /// width it cannot know. The loader converts under the position's type.
    Int(String),
    /// A float lexeme as RAW TEXT, leading `-` included (033-248).
    ///
    /// Also carries the `-inf` spelling, which is not word-shaped and so cannot
    /// arrive as an `Ident`. Bare `nan` and `inf` ARE word-shaped and arrive as
    /// `Ident`; the parser admits all three only in float-typed positions.
    Float(String),
    /// A `"…"` string's content, ESCAPES ALREADY PROCESSED (033-249).
    Str(String),
    /// A `'…'` path key's content, escapes already processed (033-243).
    QuotedKey(String),
    /// A behavior handle: the `u32` decoded from `B@` + 1–8 hex digits (033-167).
    ///
    /// A lexer-level composite (R47) — the hex run is digit-initial and so
    /// unspellable as either `Ident` or `Int`, which leaves the lexer the only
    /// layer that can read it.
    Bid(u32),
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `<` — opens a type-tag argument list (`pool_index<%Request>`).
    Lt,
    /// `>` — always ONE token. Nested tags (`portal<handle<i32>>`) close with
    /// two `Gt`s; there is no shift token to munch them into.
    Gt,
    /// `:`
    Colon,
    /// `,`
    Comma,
    /// `.` — always its own token. The lexer composes neither dotted paths nor
    /// dotted op mnemonics; the parser composes both (R42, 033-253).
    Dot,
    /// `;` — the intra-block instruction separator (§4's `InstrSep`).
    Semicolon,
    /// `=`
    Eq,
    /// `%` — the sigil of a type id (`%Request`) and of an SSA temporary (`%0`).
    Percent,
    /// `#` — the enum-variant sigil (`#Some`) and `call.dyn`'s method marker.
    Hash,
    /// `->` — the behavior signature's return arrow.
    Arrow,
    /// `@` — standalone (R47). It marks a binding's provenance
    /// (`App.label@bound`); `B@…` handles never reach here, the lexer having
    /// already read them as `Bid`.
    At,
    /// A line break. A run of them collapses to ONE token (R48, 033-250), so
    /// blank lines between entries are insignificant to the parser.
    Newline,
    /// End of input. Always the last token; the module's closing `}` is what
    /// terminates the final entry (033-250), and an unclosed section at `Eof`
    /// is an error.
    Eof,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_variants_compare_by_payload() {
        assert_eq!(
            TokenKind::Ident("count".to_string()),
            TokenKind::Ident("count".to_string())
        );
        assert_ne!(
            TokenKind::Ident("count".to_string()),
            TokenKind::Ident("show".to_string())
        );
        // Raw text, not a number: the two spellings stay distinguishable.
        assert_ne!(
            TokenKind::Int("0".to_string()),
            TokenKind::Int("-0".to_string())
        );
        assert_eq!(TokenKind::Bid(0xd1), TokenKind::Bid(209));
    }

    #[test]
    fn marks_are_distinct_from_one_another() {
        assert_ne!(TokenKind::Lt, TokenKind::Gt);
        assert_ne!(TokenKind::Newline, TokenKind::Eof);
        assert_ne!(TokenKind::Colon, TokenKind::Semicolon);
    }
}
