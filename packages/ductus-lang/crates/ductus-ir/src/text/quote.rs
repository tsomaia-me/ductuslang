//! [`quote()`] — the canonical spelling of a delimited payload, in one place.

use std::fmt::Write;

/// Write `content` into `out` as a `delimiter`-delimited payload in its ONE
/// canonical spelling (033-249, 033-243).
///
/// The two delimited payloads of the text form — a `STRING` (`"…"`) and a
/// `QuotedKey` (`'…'`) — are one grammar with the delimiter substituted, so
/// they are one emitter with the delimiter as a parameter. Raw wherever raw is
/// legal, with four exceptions:
///
/// - the delimiter escapes, because a raw one would end the lexeme;
/// - `\` escapes, because a raw one would open an escape;
/// - a carriage return emits `\r`, because a raw one would not survive
///   file-level newline normalization;
/// - any other control scalar emits `\u{…}`, because a raw one is unreadable.
///
/// A newline and a tab emit RAW. That is the owner-ruled half of the design: a
/// multi-line payload reads as multiple lines. It also means an entry carrying
/// one spans physical lines, which costs nothing — the newline is inside the
/// lexeme, so it is not the NEWLINE that terminates an entry (033-250).
///
/// Two properties follow, and both are the point. TOTALITY: every content has a
/// spelling, so no `str` value and no key content is unwritable. UNIQUENESS:
/// exactly one spelling per content, so a round trip is byte-stable. A
/// non-canonical spelling parses and is simply not what this writes.
pub(crate) fn quote(out: &mut String, delimiter: char, content: &str) {
    out.push(delimiter);

    for character in content.chars() {
        match character {
            _ if character == delimiter => {
                out.push('\\');
                out.push(delimiter);
            }
            '\\' => out.push_str("\\\\"),
            '\r' => out.push_str("\\r"),
            '\n' | '\t' => out.push(character),
            // `is_control` is Unicode's own category, so it covers the C1
            // block as well as the C0 one — and both are unreadable raw.
            _ if character.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", character as u32);
            }
            _ => out.push(character),
        }
    }

    out.push(delimiter);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quoted(delimiter: char, content: &str) -> String {
        let mut out = String::new();
        quote(&mut out, delimiter, content);

        out
    }

    // The four exceptions and the two raw cases, at both delimiters. The OTHER
    // delimiter is ordinary content in each: a `'` inside a string and a `"`
    // inside a key are raw, exactly as the two escape sets say.
    #[test]
    fn escapes_its_own_delimiter_the_backslash_and_the_carriage_return() {
        assert_eq!(quoted('"', "a\"b"), r#""a\"b""#);
        assert_eq!(quoted('\'', "it's"), r"'it\'s'");
        assert_eq!(quoted('"', "a\\b"), r#""a\\b""#);
        assert_eq!(quoted('\'', "a\\b"), r"'a\\b'");
        assert_eq!(quoted('"', "a\rb"), r#""a\rb""#);
        assert_eq!(quoted('\'', "a\rb"), r"'a\rb'");
        assert_eq!(quoted('"', "a'b"), "\"a'b\"");
        assert_eq!(quoted('\'', "a\"b"), "'a\"b'");
    }

    #[test]
    fn writes_a_newline_and_a_tab_raw_and_every_other_control_escaped() {
        assert_eq!(quoted('"', "a\nb\tc"), "\"a\nb\tc\"");
        assert_eq!(quoted('\'', "a\nb\tc"), "'a\nb\tc'");
        assert_eq!(quoted('"', "a\u{0}b"), r#""a\u{0}b""#);
        assert_eq!(quoted('\'', "\u{7}\u{1b}\u{85}"), r"'\u{7}\u{1b}\u{85}'");
    }

    #[test]
    fn writes_ordinary_content_raw_and_the_empty_payload_as_two_delimiters() {
        assert_eq!(quoted('"', "unicode é 日"), "\"unicode é 日\"");
        assert_eq!(quoted('\'', "us-east.1"), "'us-east.1'");
        assert_eq!(quoted('"', ""), "\"\"");
        assert_eq!(quoted('\'', ""), "''");
    }
}
