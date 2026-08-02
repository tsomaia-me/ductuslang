//! The path-key recognizer: `QuotedKey` (033-243).

use super::payload::{
    PAYLOAD_START, escaped_character, invalid_escape, push_line_ending, raw_run, take_raw_run,
};
use crate::diag::DiagClass;
use crate::text::lexer::scan_error::ScanError;
use crate::text::token::TokenKind;

/// A `QuotedKey` lexeme (033-243), escapes already processed.
///
/// A repeat-scope key that is not identifier-shaped quotes — `'42'`,
/// `'us-east.1'` (R41). Content is any character with exactly two escapes, `\'`
/// and `\\`; every other backslash escape is an error, so the `\n` legal inside
/// a `STRING` is not legal here.
///
/// Carriage returns follow [`string`](fn@super::string)'s rule, one law for
/// every payload (033-249): a raw `\r\n` normalizes to a lone `\n`, and a lone
/// `\r` is `lone_carriage_return`. A key wanting a literal carriage return
/// cannot spell one — `\r` is not among this lexeme's two escapes — which is no
/// loss, since a key carrying a control scalar is not identifier-shaped in the
/// first place.
pub(crate) fn quoted_key(input: &str) -> Option<Result<(usize, TokenKind), ScanError>> {
    if !input.starts_with('\'') {
        return None;
    }

    Some(scan_quoted_key(input))
}

fn scan_quoted_key(input: &str) -> Result<(usize, TokenKind), ScanError> {
    let mut value = String::with_capacity(raw_run(input, PAYLOAD_START, '\''));
    let mut cursor = PAYLOAD_START;

    loop {
        cursor = take_raw_run(input, cursor, '\'', &mut value);

        cursor = match input[cursor..].chars().next() {
            Some('\'') => return Ok((cursor + 1, TokenKind::QuotedKey(value))),
            Some('\\') => push_key_escape(input, cursor, &mut value)?,
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

fn push_key_escape(input: &str, cursor: usize, value: &mut String) -> Result<usize, ScanError> {
    let escaped = escaped_character(input, cursor)?;

    if escaped != '\'' && escaped != '\\' {
        return Err(invalid_escape(
            input,
            cursor,
            cursor + 1 + escaped.len_utf8(),
        ));
    }

    value.push(escaped);

    Ok(cursor + 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::lexer::scan::test_support::{failure, lexeme};

    #[test]
    fn quoted_key_takes_content_with_its_two_escapes() {
        assert_eq!(
            lexeme(quoted_key("'42'.render_row")),
            (4, TokenKind::QuotedKey("42".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key("'us-east.1'")),
            (11, TokenKind::QuotedKey("us-east.1".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key(r"'it\'s'")),
            (7, TokenKind::QuotedKey("it's".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key(r"'a\\b'")),
            (6, TokenKind::QuotedKey(r"a\b".to_string()))
        );
        assert_eq!(
            lexeme(quoted_key("''")),
            (2, TokenKind::QuotedKey(String::new()))
        );
        assert_eq!(quoted_key("bare"), None);
    }

    // The escape set is narrower than a string's: `\n` is a string escape and
    // nothing else.
    #[test]
    fn quoted_key_rejects_every_other_escape() {
        assert_eq!(failure(quoted_key(r"'a\nb'")), ("invalid_escape", 2, 2));
        assert_eq!(failure(quoted_key(r#"'a\"b'"#)), ("invalid_escape", 2, 2));
        assert_eq!(failure(quoted_key(r"'\u{41}'")), ("invalid_escape", 1, 2));
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
