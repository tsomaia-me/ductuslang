//! The word-shaped recognizer: `NAME` (033-168).

use super::chars::{is_ident_continue, is_ident_start};
use crate::text::token::TokenKind;

/// A word-shaped run — `NAME` (033-168): letters, digits and `_`, not starting
/// with a digit.
///
/// Letters are ASCII, pending a corpus ruling on Unicode identifiers. The
/// corpus says "letters" and pins no alphabet, so a non-ASCII letter reaches
/// the driver as an invalid character rather than being quietly admitted;
/// widening is then a ruling, not a bug fix.
///
/// Every keyword of the text form is word-shaped and comes back as `Ident`
/// (R46). The run also stops at `@`, so `App.label@bound` yields `Ident`, `At`,
/// `Ident`.
pub(crate) fn ident(input: &str) -> Option<(usize, TokenKind)> {
    let bytes = input.as_bytes();

    if !bytes.first().copied().is_some_and(is_ident_start) {
        return None;
    }

    let len = bytes
        .iter()
        .take_while(|byte| is_ident_continue(**byte))
        .count();

    Some((len, TokenKind::Ident(input[..len].to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str) -> Option<(usize, TokenKind)> {
        Some((text.len(), TokenKind::Ident(text.to_string())))
    }

    #[test]
    fn ident_takes_the_word_shaped_run_and_stops() {
        assert_eq!(ident("count"), word("count"));
        assert_eq!(ident("count : i32"), word("count"));
        assert_eq!(ident("_private_1]"), word("_private_1"));
        assert_eq!(ident("bb0:"), word("bb0"));
        assert_eq!(ident("reset_on_reopen"), word("reset_on_reopen"));
        // Stops at `@`, which leaves the provenance marker its own token.
        assert_eq!(ident("label@bound"), word("label"));
        assert_eq!(ident("pool_index<%Request>"), word("pool_index"));
    }

    #[test]
    fn ident_rejects_a_digit_first_run() {
        assert_eq!(ident("42abc"), None);
        assert_eq!(ident(""), None);
        assert_eq!(ident("-count"), None);
    }

    // ASCII pending a corpus ruling on Unicode identifiers: a non-ASCII letter
    // is not silently admitted.
    #[test]
    fn ident_rejects_non_ascii_letters() {
        assert_eq!(ident("éclair"), None);
        assert_eq!(ident("日本"), None);
        assert_eq!(ident("caf"), word("caf"));
        assert_eq!(ident("café"), word("caf"));
    }
}
