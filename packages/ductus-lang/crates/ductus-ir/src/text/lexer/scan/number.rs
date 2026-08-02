//! The numeric recognizer: `INT` (033-247) and `FLOAT` (033-248).

use super::chars::{digit_run, is_ident_continue};
use crate::text::token::TokenKind;

/// The one float lexeme that is not digit-shaped (033-248).
const NEGATIVE_INFINITY: &str = "-inf";

/// An `INT` or `FLOAT` lexeme (033-247 / 033-248), the sign inside the lexeme.
///
/// The payload is the matched text verbatim — the loader converts it under the
/// position's type, since the grammar admits literals no `i64` holds.
///
/// A `.` joins the lexeme ONLY when a digit follows it, which takes two bytes
/// of lookahead: `FLOAT` carries digits on both sides of its dot, so `42.` is
/// `Int("42")` stopping before the dot. Dotted paths (`App.print:0.text`) and
/// dotted mnemonics (`cast.i64.i32`) depend on that stop.
///
/// `-inf` matches here because the leading `-` makes it the one float spelling
/// that is not word-shaped; bare `inf` and `nan` are words and belong to
/// [`ident`](fn@super::ident). A `-` starting neither returns `None`, which
/// leaves `->` to [`punctuation`](fn@super::punctuation) and anything else to
/// the driver's invalid-character path.
pub(crate) fn number(input: &str) -> Option<(usize, TokenKind)> {
    let bytes = input.as_bytes();
    let negative = bytes.first() == Some(&b'-');

    if negative && is_negative_infinity(bytes) {
        return Some((
            NEGATIVE_INFINITY.len(),
            TokenKind::Float(NEGATIVE_INFINITY.to_string()),
        ));
    }

    let integer_start = usize::from(negative);
    let mut end = integer_start + digit_run(&bytes[integer_start..]);

    if end == integer_start {
        return None;
    }

    if bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
        end += 1;
        end += digit_run(&bytes[end..]);

        return Some((end, TokenKind::Float(input[..end].to_string())));
    }

    Some((end, TokenKind::Int(input[..end].to_string())))
}

fn is_negative_infinity(bytes: &[u8]) -> bool {
    bytes.starts_with(NEGATIVE_INFINITY.as_bytes())
        && !bytes
            .get(NEGATIVE_INFINITY.len())
            .copied()
            .is_some_and(is_ident_continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn integer(text: &str) -> Option<(usize, TokenKind)> {
        Some((text.len(), TokenKind::Int(text.to_string())))
    }

    fn float(text: &str) -> Option<(usize, TokenKind)> {
        Some((text.len(), TokenKind::Float(text.to_string())))
    }

    #[test]
    fn number_takes_integers_with_the_sign_inside() {
        assert_eq!(number("0"), integer("0"));
        assert_eq!(number("42 init"), integer("42"));
        assert_eq!(number("-5,"), integer("-5"));
        assert_eq!(
            number("170141183460469231731687303715884105727"),
            integer("170141183460469231731687303715884105727")
        );
    }

    #[test]
    fn number_takes_floats_with_digits_on_both_sides() {
        assert_eq!(number("1.5"), float("1.5"));
        assert_eq!(number("-0.25]"), float("-0.25"));
        assert_eq!(number("0.0"), float("0.0"));
    }

    // The two-byte lookahead: a `.` with no digit after it is not part of the
    // number, which is what keeps `App.print:0.text` a path.
    #[test]
    fn number_stops_before_a_dot_no_digit_follows() {
        assert_eq!(number("42."), integer("42"));
        assert_eq!(number("42.x"), integer("42"));
        assert_eq!(number("1.2.3"), float("1.2"));
    }

    #[test]
    fn number_takes_the_negative_infinity_spelling() {
        assert_eq!(number("-inf"), float("-inf"));
        assert_eq!(number("-inf,"), float("-inf"));
        assert_eq!(number("-inf]"), float("-inf"));
    }

    #[test]
    fn number_rejects_what_is_not_a_number() {
        // Word-shaped float spellings belong to `ident`.
        assert_eq!(number("inf"), None);
        assert_eq!(number("nan"), None);
        assert_eq!(number("-infinity"), None);
        assert_eq!(number("-inf_1"), None);
        // A bare `-` leaves `->` to `punctuation` and `-x` to the driver.
        assert_eq!(number("->"), None);
        assert_eq!(number("-x"), None);
        assert_eq!(number("-"), None);
        assert_eq!(number(""), None);
        assert_eq!(number(".5"), None);
    }
}
