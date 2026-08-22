//! The path-key recognizer: `QuotedKey` (033-243).

use super::payload::{PAYLOAD_START, push_escape, push_line_ending, raw_run, take_raw_run};
use crate::diag::DiagClass;
use crate::text::lexer::scan_error::ScanError;
use crate::text::token::TokenKind;

/// The `QuotedKey` delimiter (§2).
const DELIMITER: char = '\'';

/// A `QuotedKey` lexeme (033-243), escapes already processed.
///
/// A repeat-scope key that is not identifier-shaped quotes — `'42'`,
/// `'us-east.1'` — and so does one spelled by a reserved word — `'null'` (R41,
/// 002-39).
///
/// The payload is the `STRING` payload with `'` for `"` (R63): raw content is
/// any scalar but `'`, `\`, and a lone carriage return — raw newlines and raw
/// tabs ARE legal — and the escapes are `\'`, `\\`, `\n`, `\r`, `\t` and
/// `\u{1..6 hex}`. `\"` is therefore NOT an escape here, exactly as `\'` is not
/// one inside a string: the other delimiter is ordinary Raw content.
///
/// One escape grammar for both payloads is what makes key content TOTAL. Under
/// the old two-escape set `\'` and `\\`, a key carrying a carriage return had
/// no spelling at all — `Key("a\r")` emitted text that would not lex, and
/// `Key("a\r\n")` silently came back as `"a\n"`. Both round-trip now.
///
/// Carriage returns follow one law for every payload (033-249): a raw `\r\n`
/// normalizes to a lone `\n`, and a lone `\r` is `lone_carriage_return`. A key
/// wanting a literal carriage return spells `\r`, which is its canonical form.
pub(crate) fn quoted_key(input: &str) -> Option<Result<(usize, TokenKind), ScanError>> {
    if !input.starts_with(DELIMITER) {
        return None;
    }

    Some(scan_quoted_key(input))
}

fn scan_quoted_key(input: &str) -> Result<(usize, TokenKind), ScanError> {
    let mut value = String::with_capacity(raw_run(input, PAYLOAD_START, DELIMITER));
    let mut cursor = PAYLOAD_START;

    loop {
        cursor = take_raw_run(input, cursor, DELIMITER, &mut value);

        cursor = match input[cursor..].chars().next() {
            Some(DELIMITER) => return Ok((cursor + 1, TokenKind::QuotedKey(value))),
            Some('\\') => push_escape(input, cursor, DELIMITER, &mut value)?,
            Some('\r') => push_line_ending(input, cursor, &mut value)?,
            _ => {
                return Err(ScanError::new(
                    DiagClass::UnterminatedQuotedKey,
                    0,
                    1,
                    "path key reaches end of input with no closing `'`",
                ));
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::lexer::scan::test_support::{failure, lexeme};

    #[test]
    fn quoted_key_takes_raw_content_including_newlines_and_tabs() {
        assert_eq!(
            lexeme(quoted_key("'42'.render_row")),
            (4, TokenKind::QuotedKey("42".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key("'us-east.1'")),
            (11, TokenKind::QuotedKey("us-east.1".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key("'null'")),
            (6, TokenKind::QuotedKey("null".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key("'line\none\tover'")),
            (15, TokenKind::QuotedKey("line\none\tover".to_string()))
        );
        // The OTHER delimiter is ordinary Raw content, needing no escape.
        assert_eq!(
            lexeme(quoted_key("'a\"b'")),
            (5, TokenKind::QuotedKey("a\"b".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key("''")),
            (2, TokenKind::QuotedKey(String::new()))
        );
        assert_eq!(quoted_key("bare"), None);
    }

    // The full escape set, which is the STRING set with `\'` for `\"` (R63).
    // `\r` and `\u{…}` are the two that were unspellable before, and they are
    // what makes key content total.
    #[test]
    fn quoted_key_processes_every_escape() {
        assert_eq!(
            lexeme(quoted_key(r"'it\'s'")),
            (7, TokenKind::QuotedKey("it's".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key(r"'a\\b'")),
            (6, TokenKind::QuotedKey(r"a\b".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key(r"'a\nb\rc\td'")),
            (12, TokenKind::QuotedKey("a\nb\rc\td".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key(r"'\u{41}\u{1f600}'")),
            (17, TokenKind::QuotedKey("A\u{1f600}".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key(r"'\u{0}'")),
            (7, TokenKind::QuotedKey("\u{0}".to_string()))
        );
    }

    // What is left over: a spelling that names no escape at all, and the OTHER
    // delimiter's escape, which is unnecessary here exactly as `\'` is inside a
    // string. The `\u{…}` malformations are `string`'s tests, one grammar down.
    #[test]
    fn quoted_key_rejects_a_malformed_escape_at_the_escape() {
        assert_eq!(failure(quoted_key(r"'a\qb'")), ("invalid_escape", 2, 2));
        assert_eq!(failure(quoted_key(r#"'a\"b'"#)), ("invalid_escape", 2, 2));
        assert_eq!(failure(quoted_key(r"'\u{d800}'")), ("invalid_escape", 1, 8));
        assert_eq!(failure(quoted_key(r"'\u{}'")), ("invalid_escape", 1, 4));
        assert_eq!(failure(quoted_key("'a\\")), ("invalid_escape", 2, 1));
    }

    // One carriage-return law for every payload (033-249): the same two cases
    // `string` pins.
    #[test]
    fn quoted_key_normalizes_crlf_and_rejects_a_lone_carriage_return() {
        assert_eq!(
            lexeme(quoted_key("'a\r\nb'")),
            (6, TokenKind::QuotedKey("a\nb".to_string()))
        );
        assert_eq!(
            failure(quoted_key("'a\rb'")),
            ("lone_carriage_return", 2, 1)
        );
    }

    #[test]
    fn quoted_key_rejects_an_unclosed_quote() {
        assert_eq!(
            failure(quoted_key("'abc")),
            ("unterminated_quoted_key", 0, 1)
        );
        assert_eq!(failure(quoted_key("'")), ("unterminated_quoted_key", 0, 1));
    }
}
