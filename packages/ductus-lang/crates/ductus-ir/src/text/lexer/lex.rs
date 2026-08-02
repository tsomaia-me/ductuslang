//! [`lex()`] — the lexer driver: the cursor, the spans, and the dispatch (R14).

use super::scan;
use super::scan_error::ScanError;
use crate::diag::{DiagClass, Diagnostic, Span};
use crate::text::token::{Token, TokenKind};

/// The byte-order mark, stripped at offset 0 and nowhere else.
const BOM: char = '\u{feff}';

/// Lex `source` into its complete token stream, or fail at the first error.
///
/// Fail-fast (T6): the first malformed lexeme aborts the lex and returns its
/// `Diagnostic`. No recovery, no error list — `.duir` is machine-written text,
/// and a second error found after guessing at the first is noise.
///
/// The stream always ends in `Eof`. Spans are byte offsets into `source`
/// exactly as handed in: a stripped BOM shifts nothing, because stripping it
/// advances the cursor rather than rewriting the text.
///
/// A hand-written direct scanner (R13) — one loop, a match on the current byte,
/// dispatch to the pure recognizers of `super::scan`.
pub fn lex(source: &str) -> Result<Vec<Token>, Diagnostic> {
    Lexer::new(source).run()
}

/// Everything the recognizers deliberately do not have: where we are, what we
/// have produced, and how a slice-relative failure becomes a spanned one.
struct Lexer<'a> {
    source: &'a str,
    cursor: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Lexer<'a> {
        // A leading BOM is not part of the text form. Skipping it — rather than
        // rewriting the source — keeps every later span in the caller's own
        // coordinates. A BOM anywhere else is an ordinary invalid character.
        let cursor = if source.starts_with(BOM) {
            BOM.len_utf8()
        } else {
            0
        };

        Lexer {
            source,
            cursor,
            tokens: Vec::new(),
        }
    }

    fn run(mut self) -> Result<Vec<Token>, Diagnostic> {
        while let Some(byte) = self.source.as_bytes().get(self.cursor).copied() {
            match byte {
                b' ' | b'\t' => self.cursor += 1,
                b'\r' => self.skip_carriage_return()?,
                b'\n' => self.newline(),
                _ => self.lexeme()?,
            }
        }

        // `Eof` is a token and terminates a final entry (R48), so it is emitted
        // even after a trailing newline.
        self.push(TokenKind::Eof, self.cursor, 0);

        Ok(self.tokens)
    }

    // A run of newlines — blank lines, and any spaces or tabs among them —
    // collapses to ONE token spanning the run's FIRST `\n` (R48, 033-250).
    fn newline(&mut self) {
        if self.tokens.last().map(|token| &token.kind) != Some(&TokenKind::Newline) {
            self.push(TokenKind::Newline, self.cursor, 1);
        }

        self.cursor += 1;
    }

    // CRLF normalizes to LF: the `\r` of a pair carries nothing and the `\n`
    // does the newline work. A lone `\r` is a lex error — here outside a
    // lexeme, and in the delimited scanners inside one.
    fn skip_carriage_return(&mut self) -> Result<(), Diagnostic> {
        if self.source.as_bytes().get(self.cursor + 1) != Some(&b'\n') {
            return Err(self.diagnostic(
                DiagClass::LoneCarriageReturn,
                self.cursor,
                1,
                "a lone carriage return is not a line ending",
            ));
        }

        self.cursor += 1;

        Ok(())
    }

    // Dispatch order carries two rules. `bid` runs before `ident`, so `B@d1` is
    // a handle and not the name `B`; `number` runs before `punctuation`, so
    // `-5` is a literal while `->` falls through to the arrow.
    fn lexeme(&mut self) -> Result<(), Diagnostic> {
        let rest = &self.source[self.cursor..];

        let fallible = scan::string(rest)
            .or_else(|| scan::quoted_key(rest))
            .or_else(|| scan::bid(rest));

        if let Some(scanned) = fallible {
            let (len, kind) = scanned.map_err(|error| self.scan_diagnostic(error))?;
            self.accept(kind, len);

            return Ok(());
        }

        let matched = scan::ident(rest)
            .or_else(|| scan::number(rest))
            .or_else(|| scan::punctuation(rest));

        if let Some((len, kind)) = matched {
            self.accept(kind, len);

            return Ok(());
        }

        Err(self.invalid_character())
    }

    fn accept(&mut self, kind: TokenKind, len: usize) {
        self.push(kind, self.cursor, len);
        self.cursor += len;
    }

    fn push(&mut self, kind: TokenKind, start: usize, len: usize) {
        self.tokens.push(Token::new(kind, Span::new(start, len)));
    }

    // A recognizer reports offsets inside the slice it was handed; the cursor
    // is the driver's, so this is the one place the two are added.
    fn scan_diagnostic(&self, error: ScanError) -> Diagnostic {
        self.diagnostic(
            error.class,
            self.cursor + error.offset,
            error.len,
            error.message,
        )
    }

    // The offending character may be multi-byte, so the span covers the whole
    // scalar: a stray BOM points at all three of its bytes, not at one.
    fn invalid_character(&self) -> Diagnostic {
        let len = self.source[self.cursor..]
            .chars()
            .next()
            .map_or(1, char::len_utf8);

        self.diagnostic(
            DiagClass::InvalidCharacter,
            self.cursor,
            len,
            "character begins no token of the `.duir` text form",
        )
    }

    fn diagnostic(&self, class: DiagClass, start: usize, len: usize, message: &str) -> Diagnostic {
        Diagnostic {
            class,
            span: Some(Span::new(start, len)),
            path: None,
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source)
            .expect("source lexes")
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    // Class and span only — never the message (F3).
    fn failure(source: &str) -> (&'static str, Span) {
        let diagnostic = lex(source).expect_err("source fails to lex");

        (
            diagnostic.class.id(),
            diagnostic.span.expect("a lex error always carries a span"),
        )
    }

    fn word(text: &str) -> TokenKind {
        TokenKind::Ident(text.to_string())
    }

    // Byte offsets, counted by hand: `input` 0..5, `App` 6..9, `.` 9, `tick`
    // 10..14, `:` 15, `i64` 17..20, `init` 21..25, `0` 26, end 27.
    #[test]
    fn spans_are_byte_offsets_into_the_source() {
        let tokens = lex("input App.tick : i64 init 0").expect("source lexes");

        assert_eq!(
            tokens,
            vec![
                Token::new(word("input"), Span::new(0, 5)),
                Token::new(word("App"), Span::new(6, 3)),
                Token::new(TokenKind::Dot, Span::new(9, 1)),
                Token::new(word("tick"), Span::new(10, 4)),
                Token::new(TokenKind::Colon, Span::new(15, 1)),
                Token::new(word("i64"), Span::new(17, 3)),
                Token::new(word("init"), Span::new(21, 4)),
                Token::new(TokenKind::Int("0".to_string()), Span::new(26, 1)),
                Token::new(TokenKind::Eof, Span::new(27, 0)),
            ]
        );
    }

    // `a` 0, three newlines 1..4, two spaces 4..6, newline 6, space 7, `b` 8.
    #[test]
    fn a_run_of_newlines_collapses_to_one_token_at_its_first() {
        let tokens = lex("a\n\n\n  \n b").expect("source lexes");

        assert_eq!(
            tokens,
            vec![
                Token::new(word("a"), Span::new(0, 1)),
                Token::new(TokenKind::Newline, Span::new(1, 1)),
                Token::new(word("b"), Span::new(8, 1)),
                Token::new(TokenKind::Eof, Span::new(9, 0)),
            ]
        );
    }

    #[test]
    fn a_leading_byte_order_mark_is_skipped_not_rewritten() {
        let tokens = lex("\u{feff}module App").expect("source lexes");

        assert_eq!(
            tokens,
            vec![
                Token::new(word("module"), Span::new(3, 6)),
                Token::new(word("App"), Span::new(10, 3)),
                Token::new(TokenKind::Eof, Span::new(13, 0)),
            ]
        );
    }

    #[test]
    fn a_byte_order_mark_anywhere_else_is_an_invalid_character() {
        assert_eq!(
            failure("a\u{feff}b"),
            ("invalid_character", Span::new(1, 3))
        );
    }

    // `a` 0, `\r` 1, `\n` 2, `b` 3: the newline token spans the `\n`.
    #[test]
    fn crlf_normalizes_to_a_single_newline_token() {
        let tokens = lex("a\r\nb").expect("source lexes");

        assert_eq!(
            tokens,
            vec![
                Token::new(word("a"), Span::new(0, 1)),
                Token::new(TokenKind::Newline, Span::new(2, 1)),
                Token::new(word("b"), Span::new(3, 1)),
                Token::new(TokenKind::Eof, Span::new(4, 0)),
            ]
        );
    }

    #[test]
    fn a_lone_carriage_return_is_an_error_outside_and_inside_a_string() {
        assert_eq!(failure("a\rb"), ("lone_carriage_return", Span::new(1, 1)));
        // Inside a string the recognizer's slice-relative offset 2 has to be
        // added to the cursor 2 to land on the `\r` at 4.
        assert_eq!(
            failure("x \"a\rb\""),
            ("lone_carriage_return", Span::new(4, 1))
        );
    }

    #[test]
    fn a_string_keeps_its_crlf_normalization_and_escapes() {
        assert_eq!(
            kinds("\"a\r\nb\""),
            vec![TokenKind::Str("a\nb".to_string()), TokenKind::Eof]
        );
        assert_eq!(
            kinds(r#""value is \u{41}""#),
            vec![TokenKind::Str("value is A".to_string()), TokenKind::Eof]
        );
    }

    // `init` 0..4, `"` 5, `a` 6, `\` 7, `q` 8 — the escape span is driver-based
    // offset 7, not the recognizer's slice-relative 2.
    #[test]
    fn a_scan_error_lands_at_its_source_offset() {
        assert_eq!(
            failure(r#"init "a\q""#),
            ("invalid_escape", Span::new(7, 2))
        );
        assert_eq!(
            failure("uses B@123456789"),
            ("invalid_bid", Span::new(5, 11))
        );
        // A slice-relative offset of ZERO still needs the cursor added: the
        // opening quote of `x "abc` is at 2, not at 0.
        assert_eq!(failure("x \"abc"), ("unterminated_string", Span::new(2, 1)));
    }

    // The lex stops at the FIRST error: the `?` at 2, never the `!` at 6.
    #[test]
    fn the_first_error_aborts_the_lex() {
        assert_eq!(failure("a ? b !"), ("invalid_character", Span::new(2, 1)));
        assert_eq!(failure("-x -y"), ("invalid_character", Span::new(0, 1)));
    }

    #[test]
    fn eof_is_always_the_last_token() {
        assert_eq!(
            lex("").expect("empty source lexes"),
            vec![Token::new(TokenKind::Eof, Span::new(0, 0))]
        );
        assert_eq!(
            lex("\u{feff}").expect("a bare byte-order mark lexes"),
            vec![Token::new(TokenKind::Eof, Span::new(3, 0))]
        );
        assert_eq!(
            kinds("x\n"),
            vec![word("x"), TokenKind::Newline, TokenKind::Eof]
        );
        assert_eq!(kinds("\n"), vec![TokenKind::Newline, TokenKind::Eof]);
    }

    #[test]
    fn a_provenance_binding_lexes_as_ident_at_ident() {
        assert_eq!(
            kinds("params [message: App.label@bound]"),
            vec![
                word("params"),
                TokenKind::LBracket,
                word("message"),
                TokenKind::Colon,
                word("App"),
                TokenKind::Dot,
                word("label"),
                TokenKind::At,
                word("bound"),
                TokenKind::RBracket,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn a_handle_is_distinguished_from_a_word_starting_with_b() {
        assert_eq!(
            kinds("uses B@d1 Bob B"),
            vec![
                word("uses"),
                TokenKind::Bid(0xd1),
                word("Bob"),
                word("B"),
                TokenKind::Eof,
            ]
        );
    }

    // The boundary rule's payoff (033-254): `bid` declines a hex run that does
    // not end at a non-identifier byte, and the ordinary recognizers pick the
    // text up — so a provenance clause on a path segment named `B` lexes.
    #[test]
    fn a_hex_run_running_into_identifier_bytes_falls_back_to_ordinary_tokens() {
        assert_eq!(
            kinds("App.B@bound"),
            vec![
                word("App"),
                TokenKind::Dot,
                word("B"),
                TokenKind::At,
                word("bound"),
                TokenKind::Eof,
            ]
        );
        assert_eq!(
            kinds("B@d1x"),
            vec![word("B"), TokenKind::At, word("d1x"), TokenKind::Eof]
        );
    }

    // A source opening with a blank line: the newline is a token like any
    // other, not leading trivia to be skipped.
    #[test]
    fn a_leading_newline_is_the_first_token() {
        assert_eq!(
            lex("\nmodule App").expect("source lexes"),
            vec![
                Token::new(TokenKind::Newline, Span::new(0, 1)),
                Token::new(word("module"), Span::new(1, 6)),
                Token::new(word("App"), Span::new(8, 3)),
                Token::new(TokenKind::Eof, Span::new(11, 0)),
            ]
        );
    }

    #[test]
    fn signs_arrows_and_dots_stay_separable() {
        assert_eq!(
            kinds("-5 -inf 1.5 42. -> >>"),
            vec![
                TokenKind::Int("-5".to_string()),
                TokenKind::Float("-inf".to_string()),
                TokenKind::Float("1.5".to_string()),
                TokenKind::Int("42".to_string()),
                TokenKind::Dot,
                TokenKind::Arrow,
                TokenKind::Gt,
                TokenKind::Gt,
                TokenKind::Eof,
            ]
        );
    }

    /// IR_GRAMMAR §5's worked example, copied VERBATIM from the doc — the
    /// lexer's contact with real corpus text. Every module-grammar section and
    /// the behavior grammar appear here at least once.
    ///
    /// The source is the single `ductus-ir` fenced block in `docs/IR_GRAMMAR.md`
    /// §5. This constant is a COPY, not a reference, so the two drift silently:
    /// re-diff them whenever either side changes. The byte-length assertion
    /// below is the tripwire — it fires on any edit to this copy, and the
    /// re-diff is what catches an edit to the doc.
    const WORKED_EXAMPLE: &str = r#"module App {
  types { %Request = record { text: str }  size 8 align 8 }

  graph {
    scope App  exposes []  effects [App.print:0] {
      input     App.count   : i32  init 0
      input     App.show    : bool init true
      derived   App.doubled : i32  uses B@d1 inputs [App.count]
      recurrent App.total   : i32  uses B@d2 inputs [App.count] depth 1 init 0
      derived   App.label   : str  uses B@d3 inputs [App.doubled]

      gate App.g0  pred B@d4  inputs [App.show]  guards [App.print:0]

      derived   App.print:0.text    : str                  uses B@d5 inputs [App.label]
      derived   App.print:0.request : pool_index<%Request> uses B@d6 inputs [App.label]
      effect App.print:0  reconciler "print"  params [message: App.label]  desired [App.print:0.text, App.print:0.request]  gate App.g0
    }
  }

  behaviors {
    behavior B@d1 (count: i32) -> i32 {
    bb0: %0 = const.i32 2 ; %1 = mul.i32 count, %0 ; ret %1 }

    behavior B@d2 (count: i32, past1: i32) -> i32 {
    bb0: %0 = add.i32 past1, count ; ret %0 }

    behavior B@d3 (doubled: i32) -> str {
    bb0: %0 = const.str "value is " ; %1 = call int_to_str(doubled) : str
         %2 = call str_concat(move %0, move %1) : str ; ret %2 }

    behavior B@d4 (show: bool) -> bool { bb0: ret show }

    behavior B@d5 (message: str) -> str { bb0: %0 = clone message ; ret %0 }

    behavior B@d6 (message: str) -> %Request {
    bb0: %0 = record.make %Request { text: message } ; ret %0 }
  }
}
"#;

    #[test]
    fn the_worked_example_lexes_whole() {
        // Guards the verbatim copy: the doc's fenced block is 1477 bytes.
        assert_eq!(WORKED_EXAMPLE.len(), 1477);

        let tokens = lex(WORKED_EXAMPLE).expect("the worked example lexes");
        let stream: Vec<TokenKind> = tokens.iter().map(|token| token.kind.clone()).collect();

        assert_eq!(stream.len(), 415);
        assert_eq!(stream.last(), Some(&TokenKind::Eof));

        assert_eq!(
            &stream[..10],
            [
                word("module"),
                word("App"),
                TokenKind::LBrace,
                TokenKind::Newline,
                word("types"),
                TokenKind::LBrace,
                TokenKind::Percent,
                word("Request"),
                TokenKind::Eq,
                word("record"),
            ]
        );

        // `derived   App.doubled : i32  uses B@d1 …` — the handle decodes, and
        // its span covers `B@d1` at offset 262.
        assert_eq!(tokens[64].kind, TokenKind::Bid(0xd1));
        assert_eq!(tokens[64].span, Span::new(262, 4));

        // `pool_index<%Request> uses B@d6` — a `<`/`>` type-tag run.
        assert_eq!(
            &stream[155..162],
            [
                word("pool_index"),
                TokenKind::Lt,
                TokenKind::Percent,
                word("Request"),
                TokenKind::Gt,
                word("uses"),
                TokenKind::Bid(0xd6),
            ]
        );

        // The one-line effect entry, ordinal `:0` included: the colon and the
        // `0` are separate tokens, and the parser composes the path.
        assert_eq!(
            &stream[169..178],
            [
                word("effect"),
                word("App"),
                TokenKind::Dot,
                word("print"),
                TokenKind::Colon,
                TokenKind::Int("0".to_string()),
                word("reconciler"),
                TokenKind::Str("print".to_string()),
                word("params"),
            ]
        );

        // A behavior body: `bb0` is one word (the parser splits the label),
        // `%0` is two tokens, and `;` separates instructions.
        assert_eq!(
            &stream[226..236],
            [
                word("bb0"),
                TokenKind::Colon,
                TokenKind::Percent,
                TokenKind::Int("0".to_string()),
                TokenKind::Eq,
                word("const"),
                TokenKind::Dot,
                word("i32"),
                TokenKind::Int("2".to_string()),
                TokenKind::Semicolon,
            ]
        );
    }
}
