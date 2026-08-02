//! The `ScanError` struct: a recognizer's failure, in slice-relative offsets.

use crate::diag::DiagClass;

/// What a recognizer reports when the lexeme it claimed turns out malformed.
///
/// A recognizer sees only the slice it was handed and knows nothing of the
/// cursor (R14), so `offset` is relative to that slice's start. The driver adds
/// its cursor and mints the `Span` — span bookkeeping stays in exactly one
/// place.
///
/// `message` is the recognizer's own wording, kept `&'static str` so failing is
/// allocation-free. Messages are never asserted on (F3); `class` is the
/// contract.
///
/// Crate-visible, not public: this is the scan-to-driver protocol, and the
/// lexer's contract with the parser is [`Diagnostic`](crate::diag::Diagnostic)
/// alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScanError {
    /// The registry class the failure reports as.
    pub class: DiagClass,
    /// Byte offset of the failure WITHIN the recognizer's input slice.
    pub offset: usize,
    /// Length in bytes of the offending run.
    pub len: usize,
    /// Human-readable detail. Free to change; never asserted on by tests.
    pub message: &'static str,
}

impl ScanError {
    /// A failure of `class` covering `len` bytes at slice-relative `offset`.
    #[must_use]
    pub fn new(class: DiagClass, offset: usize, len: usize, message: &'static str) -> ScanError {
        ScanError {
            class,
            offset,
            len,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_carries_class_offset_and_length() {
        let error = ScanError::new(DiagClass::InvalidEscape, 4, 2, "unknown escape");

        assert_eq!(error.class.id(), "invalid_escape");
        assert_eq!(error.offset, 4);
        assert_eq!(error.len, 2);
        assert_eq!(error.message, "unknown escape");
    }
}
