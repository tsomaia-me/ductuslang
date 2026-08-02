//! The string recognizer: `STRING` (033-249).

use super::chars::hex_run;
use super::payload::{
    PAYLOAD_START, escaped_character, invalid_escape, push_line_ending, raw_run, take_raw_run,
};
use crate::diag::DiagClass;
use crate::text::lexer::scan_error::ScanError;
use crate::text::token::TokenKind;

/// A Unicode scalar escape is at most 6 hex digits (033-249).
const MAX_SCALAR_DIGITS: usize = 6;

/// A `STRING` lexeme (033-249), escapes already processed.
///
/// Raw content is any scalar but `"`, `\`, and a lone carriage return — raw
/// newlines and raw tabs ARE legal, and so is a raw control scalar, which only
/// the emitter prefers to spell `\u{…}`. Escapes are `\"`, `\\`, `\n`, `\r`,
/// `\t` and `\u{1..6 hex}`.
///
/// A raw `\r\n` inside a string normalizes to a lone `\n`, exactly as it does
/// outside one — file-level line-ending normalization cannot be selective — and
/// a lone `\r` is `lone_carriage_return` here just as it is outside. A string
/// wanting a literal carriage return spells `\r`, which is what the canonical
/// emitter writes anyway (033-249).
///
/// Two failure classes, split by what is nearer: reaching end of input with no
/// closing quote is `unterminated_string` and points at the OPENING quote;
/// anything wrong inside an escape is `invalid_escape` and points at the
/// escape — including an escape the end of input cuts short, which is a
/// malformed escape first and a missing quote second.
pub(crate) fn string(input: &str) -> Option<Result<(usize, TokenKind), ScanError>> {
    if !input.starts_with('"') {
        return None;
    }

    Some(scan_string(input))
}

fn scan_string(input: &str) -> Result<(usize, TokenKind), ScanError> {
    let mut value = String::with_capacity(raw_run(input, PAYLOAD_START, '"'));
    let mut cursor = PAYLOAD_START;

    loop {
        cursor = take_raw_run(input, cursor, '"', &mut value);

        cursor = match input[cursor..].chars().next() {
            Some('"') => return Ok((cursor + 1, TokenKind::Str(value))),
            Some('\\') => push_string_escape(input, cursor, &mut value)?,
            Some('\r') => push_line_ending(input, cursor, &mut value)?,
            _ => {
                return Err(ScanError::new(
                    DiagClass::UnterminatedString,
                    0,
                    1,
                    "string reaches end of input with no closing `\"`",
                ));
            }
        };
    }
}

fn push_string_escape(input: &str, cursor: usize, value: &mut String) -> Result<usize, ScanError> {
    let escaped = escaped_character(input, cursor)?;

    let character = match escaped {
        '"' => '"',
        '\\' => '\\',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'u' => return push_scalar_escape(input, cursor, value),
        _ => {
            return Err(invalid_escape(
                input,
                cursor,
                cursor + 1 + escaped.len_utf8(),
            ));
        }
    };

    value.push(character);

    Ok(cursor + 2)
}

// `\u{H…H}` — 1 to 6 hex digits naming a Unicode scalar. A surrogate or a value
// above 0x10FFFF names no scalar and is rejected here rather than passed on.
fn push_scalar_escape(input: &str, cursor: usize, value: &mut String) -> Result<usize, ScanError> {
    let bytes = input.as_bytes();
    let open = cursor + 2;

    if bytes.get(open) != Some(&b'{') {
        return Err(invalid_escape(input, cursor, open + 1));
    }

    let digits_start = open + 1;
    let digits = hex_run(&bytes[digits_start..]);
    let close = digits_start + digits;

    if !(1..=MAX_SCALAR_DIGITS).contains(&digits) || bytes.get(close) != Some(&b'}') {
        return Err(invalid_escape(input, cursor, close + 1));
    }

    let scalar = u32::from_str_radix(&input[digits_start..close], 16)
        .ok()
        .and_then(char::from_u32)
        .ok_or_else(|| invalid_escape(input, cursor, close + 1))?;

    value.push(scalar);

    Ok(close + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::lexer::scan::test_support::{failure, lexeme};

    #[test]
    fn string_takes_raw_content_including_newlines_and_tabs() {
        assert_eq!(
            lexeme(string(r#""print" params"#)),
            (7, TokenKind::Str("print".to_string()))
        );
        assert_eq!(lexeme(string(r#""""#)), (2, TokenKind::Str(String::new())));
        assert_eq!(
            lexeme(string("\"line\none\tover\"")),
            (15, TokenKind::Str("line\none\tover".to_string()))
        );
        // Sixteen BYTES: `é` is two and `日` is three.
        assert_eq!(
            lexeme(string("\"unicode é 日\"")),
            (16, TokenKind::Str("unicode é 日".to_string()))
        );
        assert_eq!(string("print"), None);
    }

    // A control scalar is a Raw scalar: only the EMITTER prefers `\u{…}` for it
    // (033-249), so the non-canonical spelling parses and carries the scalar.
    #[test]
    fn string_takes_a_raw_control_scalar_non_canonically() {
        assert_eq!(
            lexeme(string("\"a\u{0}b\"")),
            (5, TokenKind::Str("a\u{0}b".to_string()))
        );
        assert_eq!(
            lexeme(string("\"\u{7}\u{1b}\"")),
            (4, TokenKind::Str("\u{7}\u{1b}".to_string()))
        );
    }

    #[test]
    fn string_processes_every_escape() {
        assert_eq!(
            lexeme(string(r#""a\"b""#)),
            (6, TokenKind::Str("a\"b".to_string()))
        );
        assert_eq!(
            lexeme(string(r#""a\\b""#)),
            (6, TokenKind::Str("a\\b".to_string()))
        );
        assert_eq!(
            lexeme(string(r#""a\nb\rc\td""#)),
            (12, TokenKind::Str("a\nb\rc\td".to_string()))
        );
        assert_eq!(
            lexeme(string(r#""\u{41}\u{1f600}""#)),
            (17, TokenKind::Str("A\u{1f600}".to_string()))
        );
        assert_eq!(
            lexeme(string(r#""\u{0}""#)),
            (7, TokenKind::Str("\u{0}".to_string()))
        );
    }

    #[test]
    fn string_rejects_a_malformed_escape_at_the_escape() {
        assert_eq!(failure(string(r#""a\q""#)), ("invalid_escape", 2, 2));
        assert_eq!(failure(string(r#""\'""#)), ("invalid_escape", 1, 2));
        // Surrogate halves and out-of-range scalars name no character.
        assert_eq!(failure(string(r#""\u{d800}""#)), ("invalid_escape", 1, 8));
        assert_eq!(
            failure(string(r#""\u{110000}""#)),
            ("invalid_escape", 1, 10)
        );
        // Zero digits, seven digits, and a missing brace on either side.
        assert_eq!(failure(string(r#""\u{}""#)), ("invalid_escape", 1, 4));
        assert_eq!(
            failure(string(r#""\u{1234567}""#)),
            ("invalid_escape", 1, 11)
        );
        assert_eq!(failure(string(r#""\u41}""#)), ("invalid_escape", 1, 3));
        assert_eq!(failure(string(r#""\u{41""#)), ("invalid_escape", 1, 6));
        // Cut short by the end of input: still an escape error, clamped.
        assert_eq!(failure(string("\"a\\")), ("invalid_escape", 2, 1));
    }

    #[test]
    fn string_rejects_an_unclosed_quote_at_the_opening_quote() {
        assert_eq!(failure(string(r#""abc"#)), ("unterminated_string", 0, 1));
        assert_eq!(failure(string("\"")), ("unterminated_string", 0, 1));
        assert_eq!(failure(string("\"a\\\"b")), ("unterminated_string", 0, 1));
    }

    #[test]
    fn string_normalizes_crlf_and_rejects_a_lone_carriage_return() {
        assert_eq!(
            lexeme(string("\"a\r\nb\"")),
            (6, TokenKind::Str("a\nb".to_string()))
        );
        assert_eq!(failure(string("\"a\rb\"")), ("lone_carriage_return", 2, 1));
    }
}
