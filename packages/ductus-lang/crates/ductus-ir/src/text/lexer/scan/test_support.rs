//! Test-only helpers shared by the fallible recognizers' `tests` modules.
//!
//! Test code is private, so S1's one-public-entity law does not reach it; these
//! two live together rather than being copied into `bid`, `string` and
//! `quoted_key` alike.

use crate::text::lexer::scan_error::ScanError;
use crate::text::token::TokenKind;

pub(super) fn lexeme(scanned: Option<Result<(usize, TokenKind), ScanError>>) -> (usize, TokenKind) {
    match scanned {
        Some(Ok(matched)) => matched,
        other => panic!("expected a lexeme, got {other:?}"),
    }
}

// Class, offset and length only — never the message (F3).
pub(super) fn failure(
    scanned: Option<Result<(usize, TokenKind), ScanError>>,
) -> (&'static str, usize, usize) {
    match scanned {
        Some(Err(error)) => (error.class.id(), error.offset, error.len),
        other => panic!("expected a scan error, got {other:?}"),
    }
}
