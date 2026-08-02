//! The behavior-handle recognizer: `BID ::= 'B@' HexU32` (033-167).

use super::chars::{hex_run, is_ident_continue};
use crate::diag::DiagClass;
use crate::text::lexer::scan_error::ScanError;
use crate::text::token::TokenKind;

/// The behavior-handle prefix (033-167).
const BID_PREFIX: &str = "B@";

/// A `u32` handle is at most 8 hex digits (033-167).
const MAX_HANDLE_DIGITS: usize = 8;

/// A behavior handle — `BID ::= 'B@' HexU32` (033-167).
///
/// Only the single letter `B` IMMEDIATELY followed by `@` opens a handle: `Bob`
/// is an ordinary ident, `B` alone is an ordinary ident, and the prefix is
/// case-sensitive, so lowercase `b@d1` is no handle either (033-254).
///
/// The hex run must END at a non-identifier byte (033-254). A nonempty run that
/// runs on into identifier bytes is not a handle at all: `B@bound` and `B@d1x`
/// answer `None`, and the driver lexes `B`, `@` and the rest as ordinary
/// tokens. That is what keeps an `@`-provenance clause after a path segment
/// named `B` spellable — `App.B@bound` is `Ident`, `Dot`, `Ident`, `At`,
/// `Ident`.
///
/// A run that does end at a non-identifier byte commits to the handle: 1 to 8
/// digits decode, and any other count — none at all, or more than a `u32`
/// holds — is an error rather than a fallback to `Ident("B")`.
pub(crate) fn bid(input: &str) -> Option<Result<(usize, TokenKind), ScanError>> {
    if !input.starts_with(BID_PREFIX) {
        return None;
    }

    let bytes = input.as_bytes();
    let digits = hex_run(&bytes[BID_PREFIX.len()..]);
    let len = BID_PREFIX.len() + digits;

    if digits > 0 && bytes.get(len).copied().is_some_and(is_ident_continue) {
        return None;
    }

    if !(1..=MAX_HANDLE_DIGITS).contains(&digits) {
        return Some(Err(ScanError::new(
            DiagClass::InvalidBid,
            0,
            len,
            "a behavior handle is `B@` followed by 1 to 8 hex digits",
        )));
    }

    // Eight hex digits are exactly a `u32`, so the parse cannot fail; the arm
    // exists because `unwrap` is denied, not because the case is reachable.
    let Ok(handle) = u32::from_str_radix(&input[BID_PREFIX.len()..len], 16) else {
        return Some(Err(ScanError::new(
            DiagClass::InvalidBid,
            0,
            len,
            "behavior handle does not fit a u32",
        )));
    };

    Some(Ok((len, TokenKind::Bid(handle))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::lexer::scan::test_support::{failure, lexeme};

    #[test]
    fn bid_decodes_one_to_eight_hex_digits() {
        assert_eq!(lexeme(bid("B@d1 inputs")), (4, TokenKind::Bid(0xd1)));
        assert_eq!(lexeme(bid("B@1")), (3, TokenKind::Bid(1)));
        assert_eq!(lexeme(bid("B@aa10")), (6, TokenKind::Bid(0xaa10)));
        assert_eq!(lexeme(bid("B@ffffffff")), (10, TokenKind::Bid(u32::MAX)));
        assert_eq!(lexeme(bid("B@0000000f")), (10, TokenKind::Bid(15)));
    }

    // The run ends at a non-identifier byte in every one of these — space,
    // comma, and the end of input all close a handle.
    #[test]
    fn bid_closes_at_any_non_identifier_byte() {
        assert_eq!(lexeme(bid("B@d1 ")), (4, TokenKind::Bid(0xd1)));
        assert_eq!(lexeme(bid("B@d1,")), (4, TokenKind::Bid(0xd1)));
        assert_eq!(lexeme(bid("B@d1")), (4, TokenKind::Bid(0xd1)));
        assert_eq!(lexeme(bid("B@ffffffff,")), (10, TokenKind::Bid(u32::MAX)));
    }

    #[test]
    fn bid_rejects_a_hex_run_outside_one_to_eight() {
        assert_eq!(failure(bid("B@123456789")), ("invalid_bid", 0, 11));
        assert_eq!(failure(bid("B@")), ("invalid_bid", 0, 2));
        assert_eq!(failure(bid("B@ d1")), ("invalid_bid", 0, 2));
        assert_eq!(failure(bid("B@xyz")), ("invalid_bid", 0, 2));
    }

    // Only the single letter `B` immediately followed by `@` opens a handle.
    #[test]
    fn bid_leaves_ordinary_words_alone() {
        assert_eq!(bid("Bob"), None);
        assert_eq!(bid("B"), None);
        assert_eq!(bid("B d1"), None);
        assert_eq!(bid("b@d1"), None);
        assert_eq!(bid("label@bound"), None);
    }

    // The boundary rule (033-254): a hex run running on into identifier bytes
    // is no handle, so the driver falls back to ordinary tokens. `B@bound` is
    // the case that matters — it keeps `App.B@bound` provenance spellable.
    #[test]
    fn bid_declines_a_run_that_does_not_end_at_a_non_identifier_byte() {
        assert_eq!(bid("B@bound"), None);
        assert_eq!(bid("B@d1x"), None);
        assert_eq!(bid("B@bridge]"), None);
        assert_eq!(bid("B@constant_wrap"), None);
    }
}
